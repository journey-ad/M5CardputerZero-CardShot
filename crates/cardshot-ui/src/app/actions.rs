// 用户动作：截图、录制、删除、配置变更

use serde_json::Value;

use crate::app::poll::{load_items, poll_config, poll_status};
use crate::daemon::Client;
use crate::model::State;

pub fn clear_selected(client: &mut Client, st: &mut State) {
    delete_current(client, st);
}

pub fn clear_all(client: &mut Client, st: &mut State) {
    let removed = client.clear_all();
    st.toast(&format!("已清空 {} 项", removed));
    st.selecting = false;
    st.selected.clear();
    load_items(client, st);
    poll_status(client, st);
}

pub fn delete_current(client: &mut Client, st: &mut State) {
    let ids: Vec<String> = if st.selecting && !st.selected.is_empty() {
        let mut picked: Vec<usize> = st.selected.iter().copied().collect();
        picked.sort_unstable();
        picked
            .iter()
            .filter_map(|item_index| st.items.get(*item_index).map(|it| it.id.clone()))
            .collect()
    } else {
        st.items
            .get(st.grid_focus)
            .map(|it| vec![it.id.clone()])
            .unwrap_or_default()
    };
    if ids.is_empty() {
        return;
    }
    let removed = client.delete(&ids);
    st.toast(&format!("已删除 {} 项", removed));
    st.selecting = false;
    st.selected.clear();
    load_items(client, st);
    poll_status(client, st);
}

pub fn apply_config(client: &mut Client, st: &mut State, patch: Value) {
    client.config_set(patch);
    poll_config(client, st);
    poll_status(client, st);
}

pub fn toggle_remote(client: &mut Client, st: &mut State) {
    let enable = !st.remote_running;
    client.set_remote(enable);
    poll_status(client, st);
    poll_config(client, st);
}
