use super::*;

fn bytes(hex: &str) -> Vec<u8> {
    if hex == "-" {
        return Vec::new();
    }
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn array<const N: usize>(hex: &str) -> [u8; N] {
    bytes(hex).try_into().unwrap()
}

#[test]
fn monocypher_known_answers() {
    let mut writer = Aead::new(&[0; 32], &[0; 24]);
    let mut reader = writer.clone();
    for line in include_str!("vectors.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let p: Vec<_> = line.split_whitespace().collect();
        match p[0] {
            "x" => assert_eq!(x25519(&array(p[1]), &array(p[2])), array::<32>(p[3])),
            "public" => assert_eq!(public_key(&array(p[1])), array::<32>(p[2])),
            "hash" => {
                let size = p[1].parse().unwrap();
                let message = bytes(p[2]);
                // Split across the 128-byte BLAKE2b block boundary, including empty updates.
                for chunk in [1, 3, 127, 128, 129, 1024] {
                    let mut hash = Hash::new(size);
                    hash.update(&[]);
                    for part in message.chunks(chunk) {
                        hash.update(part);
                    }
                    let mut output = vec![0; size];
                    hash.finish(&mut output);
                    assert_eq!(output, bytes(p[3]));
                }
            }
            "kdf" => {
                let secret = array(p[1]);
                let peer_secret = array(p[2]);
                let public = public_key(&secret);
                let peer = public_key(&peer_secret);
                let mut keys = [0; 64];
                assert!(exchange(
                    &mut keys,
                    &secret,
                    &public,
                    &peer,
                    X25519KeyExchangeSide::SERVER,
                    &bytes(p[3])
                ));
                assert_eq!(keys, array::<64>(p[4]));
                assert!(exchange(
                    &mut keys,
                    &peer_secret,
                    &peer,
                    &public,
                    X25519KeyExchangeSide::CLIENT,
                    &bytes(p[3])
                ));
                assert_eq!(keys, array::<64>(p[4]));
                assert!(!exchange(
                    &mut keys,
                    &secret,
                    &public,
                    &[0; 32],
                    X25519KeyExchangeSide::SERVER,
                    &[]
                ));
                assert_eq!(keys, array::<64>(p[4]));
            }
            "stream" => {
                writer = Aead::new(&array(p[1]), &array(p[2]));
                reader = writer.clone();
            }
            "message" => {
                let plain = bytes(p[1]);
                let ad = bytes(p[2]);
                let expected = bytes(p[3]);
                let mac = array(p[4]);
                let mut cipher = plain.clone();
                assert_eq!(writer.write(&mut cipher, &ad), mac);
                assert_eq!(cipher, expected);
                let mut bad = mac;
                bad[0] ^= 1;
                assert!(!reader.read(&mut cipher, &bad, &ad));
                assert_eq!(cipher, expected);
                // Failure preserves key and nonce; retry succeeds on the same owner.
                assert!(reader.read(&mut cipher, &mac, &ad));
                assert_eq!(cipher, plain);
            }
            "lock" => {
                let mut context = Aead::new(&array(p[1]), &array(p[2]));
                let mut text = bytes(p[3]);
                assert_eq!(context.write(&mut text, &bytes(p[4])), array::<16>(p[6]));
                assert_eq!(text, bytes(p[5]));
                assert!(Aead::new(&array(p[1]), &array(p[2])).read(
                    &mut text,
                    &array(p[6]),
                    &bytes(p[4])
                ));
                assert_eq!(text, bytes(p[3]));
            }
            "sha512" => assert_eq!(Sha512::digest(bytes(p[1])).as_slice(), bytes(p[2])),
            "ed" => assert_eq!(
                ed25519_check(&array(p[2]), &array(p[3]), &bytes(p[4])),
                p[1] == "1",
                "{line}"
            ),
            _ => panic!("unknown vector: {line}"),
        }
    }
}
