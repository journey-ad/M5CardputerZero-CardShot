import { html, useState, useEffect, Icon, api, translate } from "../core.js";

function Settings({ cfg, toast, onSaved, onLangPreview }) {
  const [draft, setDraft] = useState(null);
  const [saving, setSaving] = useState(false);
  useEffect(() => { setDraft(JSON.parse(JSON.stringify(cfg))); }, [cfg]);
  if (!draft) return null;
  const updateDraft = (patch) => setDraft((previous) => ({ ...previous, ...patch }));
  const updateRecord = (patch) => setDraft((previous) => ({ ...previous, record: { ...previous.record, ...patch } }));
  const updateService = (patch) => setDraft((previous) => ({ ...previous, service: { ...(previous.service || {}), ...patch } }));
  const save = async () => {
    setSaving(true);
    try {
      const patch = {
        language: draft.language,
        service: { autostart: !!(draft.service && draft.service.autostart) },
        image_format: draft.image_format,
        image_quality: Number(draft.image_quality),
        record: { fps: Number(draft.record.fps) },
      };
      const response = await api("/api/config", { method: "POST", body: JSON.stringify({ patch }) });
      if (response.ok) { toast(translate("saved")); onSaved && onSaved(); }
      else toast(response.error || translate("save_failed"), true);
    } catch (error) { toast(translate("save_failed") + "：" + error.message, true); }
    setSaving(false);
  };
  const clearAll = async () => {
    if (!window.confirm(translate("clear_confirm"))) return;
    try {
      const list = await api("/api/items");
      const ids = (list.items || []).map((it) => it.id);
      if (!ids.length) { toast(translate("empty")); return; }
      const response = await api("/api/delete", { method: "POST", body: JSON.stringify({ ids }) });
      if (response.ok) { toast(translate("deleted") + (response.removed != null ? response.removed : ids.length) + translate("deleted_suf")); onSaved && onSaved(); }
      else toast(response.error || translate("delete_failed"), true);
    } catch (error) { toast(translate("delete_failed") + "：" + error.message, true); }
  };
  const autostart = !!(draft.service && draft.service.autostart);
  const langs = [["zh", "中文"], ["en", "English"], ["ja", "日本語"]];
  return html`<div>
    <div class="settings-grid">
      <div class="card">
        <div class="card-head"><div class="card-title"><${Icon} name="settings" size=${16} />${translate("common")}</div></div>
        <div class="card-body">
          <div class="field-row">
            <label class="label">${translate("autostart")}</label>
            <div class=${"switch " + (autostart ? "on" : "")} onClick=${() => updateService({ autostart: !autostart })}><i></i></div>
          </div>
          <div class="field">
            <label class="label">${translate("language")}</label>
            <div class="seg">
              ${langs.map(([code, name]) => html`<button class=${draft.language === code ? "active" : ""}
                onClick=${() => { updateDraft({ language: code }); onLangPreview && onLangPreview(code); }}>${name}</button>`)}
            </div>
          </div>
          <div class="field">
            <label class="label">${translate("image_format")}</label>
            <div class="seg">
              ${["png", "jpeg", "webp"].map((format) => html`<button class=${draft.image_format === format ? "active" : ""}
                onClick=${() => updateDraft({ image_format: format })}>${format.toUpperCase()}</button>`)}
            </div>
          </div>
          <div class="field">
            <label class="label">${translate("quality")}</label>
            <div class="seg">
              ${[60, 75, 85, 90, 95, 100].map((quality) => html`<button class=${Number(draft.image_quality) === quality ? "active" : ""}
                onClick=${() => updateDraft({ image_quality: quality })}>${quality}</button>`)}
            </div>
            <div class="hint">${translate("quality_hint")}</div>
          </div>
          <div class="field">
            <label class="label">${translate("fps")}</label>
            <div class="seg">
              ${[10, 15, 20, 25, 30].map((fpsValue) => html`<button class=${Number(draft.record.fps) === fpsValue ? "active" : ""}
                onClick=${() => updateRecord({ fps: fpsValue })}>${fpsValue}</button>`)}
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-head"><div class="card-title"><${Icon} name="database" size=${16} />${translate("data")}</div></div>
        <div class="card-body">
          <div class="field">
            <label class="label">${translate("storage")}</label>
            <div class="hint">${translate("storage_hint")}</div>
          </div>
          <div class="field-row">
            <label class="label">${translate("clear")}</label>
            <button class="btn btn-destructive btn-sm" onClick=${clearAll}><${Icon} name="trash-2" size=${15} />${translate("clear_btn")}</button>
          </div>
        </div>
      </div>
    </div>
    <div class="settings-actions">
      <button class="btn btn-outline" onClick=${() => setDraft(JSON.parse(JSON.stringify(cfg)))}>${translate("reset")}</button>
      <button class="btn btn-primary" disabled=${saving} onClick=${save}>
        ${saving ? html`<${Icon} name="refresh-cw" size=${16} cls="spin" />${translate("saving")}` : html`<${Icon} name="check" size=${16} />${translate("save")}`}
      </button>
    </div>
  </div>`;
}

export { Settings };
