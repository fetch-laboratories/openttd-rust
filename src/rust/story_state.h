/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file story_state.h Native loader aliases into process-wide Rust story control state. */
#ifndef RUST_STORY_STATE_H
#define RUST_STORY_STATE_H
struct OpenTTDStoryState {
	uint32_t page_next_sort;
	uint32_t element_next_sort;
};
extern "C" OpenTTDStoryState *openttd_rust_story_storage();
inline OpenTTDStoryState &GetRustStoryState() noexcept
{
	static OpenTTDStoryState *state = new (openttd_rust_story_storage()) OpenTTDStoryState{};
	return *state;
}
#endif /* RUST_STORY_STATE_H */
