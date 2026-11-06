/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Map-transfer payloads. Native save/load filters retain their shared ownership,
//! mutex, condition variable and throwing frames. Writer access requires its native
//! mutex; readers follow the load-filter lifetime. No payload alias crosses a callback.
//! Panics abort; native exceptions stay on the filter's C++ frame.
#![allow(unsafe_code, missing_docs)]
use crate::industrytype::NativeVec;
use crate::network_config::TCP_MTU;
use crate::network_tcp::{self as tcp, Packet, Services};
use crate::packet::NetworkSocketHandler;
use crate::tcp_game::{PACKET_SERVER_MAP_DATA, PACKET_SERVER_MAP_DONE, PACKET_SERVER_MAP_SIZE};
use std::ffi::c_void;
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDNetworkMapWriter" ["network_map"] => pub struct MapWriter {
        pub resource: *mut c_void,
        pub client: *mut NetworkSocketHandler,
        pub current: *mut Packet => "Packet *",
        pub total_size: usize,
        pub next_packet: usize,
        pub packets: NativeVec<*mut Packet> => "RustVector<Packet *>",
    }
    "OpenTTDNetworkMapReader" ["network_map"] => pub struct MapReader {
        pub buffer: NativeVec<u8> => "RustVector<uint8_t>",
        pub read_bytes: usize,
    }
}

impl MapWriter {
    fn new(resource: *mut c_void, client: *mut NetworkSocketHandler) -> Self {
        Self {
            resource,
            client,
            current: null_mut(),
            total_size: 0,
            next_packet: 0,
            packets: NativeVec::new(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_map_writer_new(
    resource: *mut c_void,
    client: *mut NetworkSocketHandler,
) -> *mut MapWriter {
    Box::into_raw(Box::new(MapWriter::new(resource, client)))
}

/// # Safety
/// The native filter has observed a null client under its mutex; `writer` is uniquely owned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_writer_drop(
    services: &Services,
    writer: *mut MapWriter,
) {
    unsafe {
        for i in (*writer).next_packet..(*writer).packets.len() {
            tcp::destroy_packet(services, *(*writer).packets.as_mut_ptr().add(i));
        }
        if !(*writer).current.is_null() {
            tcp::destroy_packet(services, (*writer).current);
        }
        drop(Box::from_raw(writer));
    }
}

/// # Safety
/// The caller holds the writer mutex, has handled a null client with `SlError`, and supplies
/// `size` live bytes (null is permitted when empty). Services follow the packet contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_write(
    writer: *mut MapWriter,
    services: &Services,
    check: extern "C" fn(bool),
    data: *const u8,
    size: usize,
) {
    unsafe {
        if (*writer).current.is_null() {
            (*writer).current = tcp::new_outgoing_shell(
                services,
                (*writer).client,
                PACKET_SERVER_MAP_DATA,
                TCP_MTU,
                check,
            );
        }
        let mut remaining = if size == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(data, size)
        };
        while !remaining.is_empty() {
            let current = tcp::record((*writer).current);
            remaining = (&mut *current).send_bytes(remaining);
            if !(*current).can_write(1) {
                let current = std::mem::replace(&mut (*writer).current, null_mut());
                (*writer).packets.push(current);
                if !remaining.is_empty() {
                    (*writer).current = tcp::new_outgoing_shell(
                        services,
                        (*writer).client,
                        PACKET_SERVER_MAP_DATA,
                        TCP_MTU,
                        check,
                    );
                }
            }
        }
        (*writer).total_size = (*writer).total_size.wrapping_add(size);
    }
}

/// # Safety
/// The writer is live, its mutex is held and its client is nonnull; packet services are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_finish(
    writer: *mut MapWriter,
    services: &Services,
    check: extern "C" fn(bool),
) {
    unsafe {
        if !(*writer).current.is_null() {
            let current = std::mem::replace(&mut (*writer).current, null_mut());
            (*writer).packets.push(current);
        }
        let client = (*writer).client;
        let done = tcp::new_outgoing_shell(
            services,
            client,
            PACKET_SERVER_MAP_DONE,
            crate::network_config::COMPAT_MTU,
            check,
        );
        (*writer).packets.push(done);
        let size = tcp::new_outgoing_shell(
            services,
            client,
            PACKET_SERVER_MAP_SIZE,
            crate::network_config::COMPAT_MTU,
            check,
        );
        #[allow(clippy::cast_possible_truncation)] // PacketWriter narrows its native-size total.
        let total = (*writer).total_size as u32;
        (*tcp::record(size)).send_bytes(&total.to_le_bytes());
        (*writer).packets.insert(0, size);
    }
}

impl MapReader {
    #[allow(clippy::cast_possible_truncation)] // The join-progress field is a native uint32_t.
    pub(crate) fn append(&mut self, bytes: &[u8], check: extern "C" fn(bool)) -> u32 {
        check(self.read_bytes == 0);
        self.buffer.edit(|buffer| buffer.extend_from_slice(bytes));
        self.buffer.len() as u32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_map_reader_new() -> *mut MapReader {
    Box::into_raw(Box::new(MapReader {
        buffer: NativeVec::new(),
        read_bytes: 0,
    }))
}

/// # Safety
/// The final native filter owner releases its uniquely owned live reader.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_reader_drop(reader: *mut MapReader) {
    unsafe { drop(Box::from_raw(reader)) };
}

/// # Safety
/// The live load-filter reader is exclusively accessible during Reset.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_reset(reader: *mut MapReader) {
    unsafe { (*reader).read_bytes = 0 };
}

/// # Safety
/// The reader and `size` writable output bytes are live and disjoint; the load frame
/// serializes reads and Reset. No buffer mutation occurs during a read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_map_read(
    reader: *mut MapReader,
    output: *mut u8,
    size: usize,
) -> usize {
    unsafe {
        let count = (*reader)
            .buffer
            .len()
            .wrapping_sub((*reader).read_bytes)
            .min(size);
        if count != 0 {
            std::ptr::copy_nonoverlapping(
                (*reader).buffer.as_mut_ptr().add((*reader).read_bytes),
                output,
                count,
            );
        }
        (*reader).read_bytes += count;
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn check(value: bool) {
        assert!(value);
    }

    #[test]
    fn map_packets_preserve_size_priority_fragmentation_and_empty_write() {
        for chunks in [
            vec![],
            vec![0],
            vec![1],
            vec![32764],
            vec![32765],
            vec![17, 65534],
        ] {
            let total: usize = chunks.iter().sum();
            let bytes: Vec<u8> = (0..=u8::MAX).cycle().take(total).collect();
            let services = tcp::packet_only_services();
            let mut base = NetworkSocketHandler::new();
            let mut writer = Box::new(MapWriter::new(null_mut(), &raw mut base));
            let mut offset = 0;
            for &size in &chunks {
                unsafe {
                    openttd_rust_network_map_write(
                        &raw mut *writer,
                        &services,
                        check,
                        bytes.as_ptr().add(offset),
                        size,
                    );
                }
                offset += size;
            }
            unsafe {
                openttd_rust_network_map_finish(&raw mut *writer, &services, check);
            };
            let packets = writer.packets.as_slice();
            let wire = |p: &*mut Packet| unsafe {
                let p = tcp::record(*p);
                std::slice::from_raw_parts((*p).data, (*p).size)
            };
            let mut size_packet = vec![0, 0, PACKET_SERVER_MAP_SIZE];
            size_packet.extend_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
            assert_eq!(wire(&packets[0]), size_packet);
            assert_eq!(
                wire(packets.last().unwrap()),
                [0, 0, PACKET_SERVER_MAP_DONE]
            );
            let mut data = Vec::new();
            for p in &packets[1..packets.len() - 1] {
                assert!(unsafe { (*tcp::record(*p)).size } <= 32767);
                assert_eq!(&wire(p)[..3], [0, 0, PACKET_SERVER_MAP_DATA]);
                data.extend_from_slice(&wire(p)[3..]);
            }
            assert_eq!(data, bytes);
            assert_eq!(
                packets.len(),
                2 + if chunks.is_empty() {
                    0
                } else {
                    total.div_ceil(32764).max(1)
                }
            );
            assert_eq!(writer.total_size, total);
            assert!(writer.current.is_null());
            writer.client = null_mut();
            unsafe {
                openttd_rust_network_map_writer_drop(&services, Box::into_raw(writer));
            }
        }
    }

    #[test]
    fn reader_rewinds_and_bounds_reads_without_consuming_source_bytes() {
        let mut reader = MapReader {
            buffer: NativeVec::new(),
            read_bytes: 0,
        };
        assert_eq!(reader.append(&[9, 8], check), 2);
        assert_eq!(reader.append(&[7, 6, 5], check), 5);
        let mut output = [0; 8];
        unsafe {
            assert_eq!(
                openttd_rust_network_map_read(&raw mut reader, output.as_mut_ptr(), 2),
                2
            );
            assert_eq!(&output[..2], [9, 8]);
            assert_eq!(
                openttd_rust_network_map_read(&raw mut reader, output.as_mut_ptr(), 8),
                3
            );
            assert_eq!(&output[..3], [7, 6, 5]);
            assert_eq!(
                openttd_rust_network_map_read(&raw mut reader, output.as_mut_ptr(), 1),
                0
            );
            openttd_rust_network_map_reset(&raw mut reader);
            assert_eq!(
                openttd_rust_network_map_read(&raw mut reader, output.as_mut_ptr(), 8),
                5
            );
        }
        assert_eq!(&output[..5], [9, 8, 7, 6, 5]);
        assert_eq!(reader.buffer.as_slice(), [9, 8, 7, 6, 5]);
    }
}
