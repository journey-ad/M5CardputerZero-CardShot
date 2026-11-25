use super::*;

pub(crate) fn handle_preview(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    match key {
        "LEFT" => {
            if st.grid_focus > 0 {
                st.grid_focus -= 1;
                open_preview(ui, client, st);
            }
        }
        "RIGHT" => {
            if st.grid_focus + 1 < st.items.len() {
                st.grid_focus += 1;
                open_preview(ui, client, st);
            }
        }
        "ENTER" | "SPACE" => {
            if st.video_ended && st.preview_kind == "video" {
                st.video_ended = false;
                open_preview(ui, client, st);
            }
        }
        "BACK" => {
            st.confirm_kind = crate::constants::CONFIRM_DELETE;
            st.confirm_focus = 0;
            st.confirm_visible = true;
        }
        "SHIFT_BACK" => {
            delete_current(client, st);
            st.page = Page::Gallery;
        }
        _ => {}
    }
}
