use super::*;

pub(crate) fn handle_gallery(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    let len = st.items.len();
    match key {
        "LEFT" => {
            if st.grid_focus > 0 {
                st.grid_focus -= 1;
            }
        }
        "RIGHT" => {
            if st.grid_focus + 1 < len {
                st.grid_focus += 1;
            }
        }
        "UP" => {
            if st.grid_focus >= GRID_COLS {
                st.grid_focus -= GRID_COLS;
            }
        }
        "DOWN" => {
            if st.grid_focus + GRID_COLS < len {
                st.grid_focus += GRID_COLS;
            }
        }
        "SPACE" => {
            st.selecting = true;
            if st.selected.contains(&st.grid_focus) {
                st.selected.remove(&st.grid_focus);
            } else {
                st.selected.insert(st.grid_focus);
            }
        }
        "ENTER" => open_preview(ui, client, st),
        "BACK" => {
            st.confirm_kind = crate::constants::CONFIRM_DELETE;
            st.confirm_focus = 0;
            st.confirm_visible = true;
        }
        "SHIFT_BACK" => {
            if st.selecting && st.selected.len() > 1 {
                clear_selected(client, st);
            } else {
                delete_current(client, st);
            }
        }
        _ => {}
    }
}
