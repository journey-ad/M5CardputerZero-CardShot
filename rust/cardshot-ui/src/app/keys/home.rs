use super::*;

pub(crate) fn handle_home(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    let count = HomeAction::LIST.len();
    match key {
        "LEFT" => {
            if st.home_focus % HOME_COLS > 0 {
                st.home_focus -= 1;
            }
        }
        "RIGHT" => {
            if st.home_focus % HOME_COLS + 1 < HOME_COLS && st.home_focus + 1 < count {
                st.home_focus += 1;
            }
        }
        "UP" => {
            if st.home_focus >= HOME_COLS {
                st.home_focus -= HOME_COLS;
            }
        }
        "DOWN" => {
            if st.home_focus + HOME_COLS < count {
                st.home_focus += HOME_COLS;
            }
        }
        "ENTER" => match HomeAction::at(st.home_focus) {
            HomeAction::Gallery => {
                load_items(client, st);
                st.page = Page::Gallery;
            }
            HomeAction::Settings => {
                poll_config(client, st);
                st.page = Page::Settings;
            }
            HomeAction::Help => st.help_visible = true,
        },
        _ => {}
    }
    let _ = ui;
}
