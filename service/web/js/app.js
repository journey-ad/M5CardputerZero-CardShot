import {
  html, useState, useEffect, useRef, useCallback,
  api, translate, setLang, Icon, humanSize, mmss, fmtDate, fileUrl, Thumb,
} from "./core.js";
import { GalleryView } from "./components/gallery.js";

function App() {
  const [tab, setTab] = useState("gallery");
  const [status, setStatus] = useState(null);
  const [items, setItems] = useState([]);
  const [cfg, setCfg] = useState(null);
  const [sel, setSel] = useState(() => ({ mode: false, ids: new Set() }));
  const [view, setView] = useState("grid");
  const [pageSize, setPageSize] = useState(12);
  const [curPage, setCurPage] = useState(1);
  const [marquee, setMarquee] = useState(null);
  const gridRef = useRef(null);
  const galleryRef = useRef(null);
  const dragRef = useRef(null);
  const suppressClick = useRef(false);
  const [light, setLight] = useState(-1);
  const [zoom, setZoom] = useState(1);
  const [toastMsg, setToastMsg] = useState(null);
  const [busy, setBusy] = useState(false);
  const toastTimer = useRef(null);

  const toast = useCallback((msg, isErr) => {
    setToastMsg({ msg, err: !!isErr });
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToastMsg(null), 2600);
  }, []);

  const refreshStatus = useCallback(async () => {
    try { const response = await api("/api/status"); if (response.ok) setStatus(response.status); } catch (error) {}
  }, []);
  const refreshItems = useCallback(async () => {
    try { const response = await api("/api/items"); if (response.items) setItems(response.items); } catch (error) {}
  }, []);
  const refreshConfig = useCallback(async () => {
    try { const response = await api("/api/config"); if (response.config) setCfg(response.config); } catch (error) {}
  }, []);

  useEffect(() => {
    refreshStatus(); refreshItems(); refreshConfig();
    const t1 = setInterval(refreshStatus, 1500);
    const t2 = setInterval(refreshItems, 5000);
    return () => { clearInterval(t1); clearInterval(t2); };
  }, []);

  const dragHandlers = useRef({});
  useEffect(() => { dragHandlers.current = { down: onGridDown, move: onGridMove, up: onGridUp }; });
  useEffect(() => {
    if (tab !== "gallery") return;
    const down = (event) => {
      if (event.button !== 0) return;
      if (event.target.closest("button,select,a,input,.tabs,.lb-bar,.lightbox,.topbar,.hero,.toast,.marquee")) return;
      dragHandlers.current.down(event);
    };
    const move = (event) => dragHandlers.current.move(event);
    const up = (event) => dragHandlers.current.up(event);
    document.addEventListener("mousedown", down, true);
    document.addEventListener("mousemove", move, true);
    document.addEventListener("mouseup", up, true);
    return () => {
      document.removeEventListener("mousedown", down, true);
      document.removeEventListener("mousemove", move, true);
      document.removeEventListener("mouseup", up, true);
    };
  }, [tab]);

  const doShot = async () => {
    setBusy(true);
    try {
      const response = await api("/api/shot", { method: "POST", body: "{}" });
      if (response.ok) { toast(translate("shot_ok") + (response.name || "")); refreshStatus(); refreshItems(); }
      else toast(response.error || translate("shot_failed"), true);
    } catch (error) { toast(translate("shot_failed") + "：" + error.message, true); }
    setBusy(false);
  };
  const doRecord = async (action) => {
    setBusy(true);
    try {
      const response = await api("/api/record", { method: "POST", body: JSON.stringify({ action }) });
      if (response.ok) { toast(action === "stop" ? translate("rec_stopped") : translate("rec_started")); refreshStatus(); refreshItems(); }
      else toast(response.error || translate("op_failed"), true);
    } catch (error) { toast(translate("op_failed") + "：" + error.message, true); }
    setBusy(false);
  };
  const doDelete = async (ids) => {
    if (!ids.length) return;
    if (!window.confirm(translate("confirm_delete").replace("{n}", ids.length))) return;
    try {
      const response = await api("/api/delete", { method: "POST", body: JSON.stringify({ ids }) });
      if (response.ok) {
        toast(translate("deleted") + (response.removed != null ? response.removed : ids.length) + translate("deleted_suf"));
        setLight(-1);
        setSel({ mode: false, ids: new Set() });
        refreshItems(); refreshStatus();
      } else toast(response.error || translate("delete_failed"), true);
    } catch (error) { toast(translate("delete_failed") + "：" + error.message, true); }
  };

  setLang((cfg && cfg.language) || "zh");
  const totalPages = Math.max(1, Math.ceil(items.length / pageSize));
  const page = Math.min(curPage, totalPages);
  const pageItems = items.slice((page - 1) * pageSize, page * pageSize);
  const rec = status && status.recording;
  const counts = (status && status.counts) || { images: 0, videos: 0 };
  const selIds = Array.from(sel.ids);

  const toggleSel = (id) => setSel((state) => {
    const next = new Set(state.ids); next.has(id) ? next.delete(id) : next.add(id); return { ...state, ids: next };
  });
  const pickItem = (item, itemIndex) => {
    if (suppressClick.current) { suppressClick.current = false; return; }
    sel.mode ? toggleSel(item.id) : setLight(itemIndex);
  };
  const onGridDown = (event) => {
    if (event.button !== 0) return;
    document.body.style.userSelect = "none";
    if (event.target.closest(".toolbar,button,select,a,input,.tabs")) return;
    dragRef.current = { startX: event.clientX, startY: event.clientY, base: new Set(sel.ids), moved: false };
    suppressClick.current = false;
    setMarquee({ x: event.clientX, y: event.clientY, width: 0, height: 0 });
  };
  const onGridMove = (event) => {
    const drag = dragRef.current;
    if (!drag) return;
    if (Math.abs(event.clientX - drag.startX) > 3 || Math.abs(event.clientY - drag.startY) > 3) drag.moved = true;
    setMarquee({ x: Math.min(event.clientX, drag.startX), y: Math.min(event.clientY, drag.startY),
      width: Math.abs(event.clientX - drag.startX), height: Math.abs(event.clientY - drag.startY) });
  };
  const onGridUp = (event) => {
    const drag = dragRef.current;
    if (!drag) return;
    dragRef.current = null;
    setMarquee(null);
    document.body.style.userSelect = "";
    if (!drag.moved || !galleryRef.current) return;
    suppressClick.current = true;
    const box = { left: Math.min(event.clientX, drag.startX), top: Math.min(event.clientY, drag.startY),
      right: Math.max(event.clientX, drag.startX), bottom: Math.max(event.clientY, drag.startY) };
    const ids = new Set(drag.base);
    galleryRef.current.querySelectorAll("[data-tid]").forEach((element) => {
      const rect = element.getBoundingClientRect();
      const hit = !(rect.right < box.left || rect.left > box.right || rect.bottom < box.top || rect.top > box.bottom);
      if (hit) ids.add(element.getAttribute("data-tid"));
    });
    if (ids.size > 0) setSel({ mode: true, ids });
  };
  const enterSelect = (id) => setSel({ mode: true, ids: new Set(id ? [id] : []) });

  const selectAll = () => setSel({ mode: true, ids: new Set(items.map((item) => item.id)) });

  const [SettingsComponent, setSettingsComponent] = useState(null);
  const [LightboxComponent, setLightboxComponent] = useState(null);
  useEffect(() => {
    let active = true;
    import("./components/settings.js").then((module) => { if (active) setSettingsComponent(() => module.Settings); });
    import("./components/lightbox.js").then((module) => { if (active) setLightboxComponent(() => module.Lightbox); });
    return () => { active = false; };
  }, []);

  const galleryContext = {
    items, pageItems, totalPages, page, pageSize, view, sel, selIds, marquee,
    galleryRef, gridRef, translate, Icon, Thumb, humanSize, mmss, fmtDate,
    setView, setPageSize, setCurPage, selectAll, enterSelect, doDelete, setSel, pickItem,
  };

  return html`<div class="container">
    <div class="topbar">
      <div class="brand">
        <div class="brand-logo"><${Icon} name="camera" size=${22} /></div>
        <div>
          <div class="brand-title">CardShot</div>
          <div class="brand-sub">${translate("brand_sub")}</div>
        </div>
      </div>
      <div class="spacer"></div>
      
      <button class="btn btn-ghost btn-icon" title=${translate("refresh")} onClick=${() => { refreshStatus(); refreshItems(); }}>
        <${Icon} name="refresh-cw" size=${18} /></button>
    </div>

    <div class="card hero">
      <div class="hero-main">
        <div class=${"rec-badge " + (rec ? "on" : "")}>
          <${Icon} name=${rec ? "square" : "circle"} size=${22} />
        </div>
        <div>
          <div class="hero-title">${rec ? translate("recording") : translate("idle")}</div>
          <div class="hero-sub">${rec
            ? html`<span class="mono">${mmss(status.elapsed)}</span> · ${status.frames} ${translate("frames")} · ${status.record_fps || (cfg && cfg.record.fps) || 20} FPS`
            : translate("hint_idle")}</div>
        </div>
      </div>
      <div class="stat-row">
        <div class="stat"><div class="stat-k">${translate("stat_shots")}</div><div class="stat-v">${counts.images}</div></div>
        <div class="stat"><div class="stat-k">${translate("stat_videos")}</div><div class="stat-v">${counts.videos}</div></div>
        <div class="stat"><div class="stat-k">${translate("stat_size")}</div><div class="stat-v">${humanSize(status && status.total_bytes)}</div></div>
        <div class="stat"><div class="stat-k">${translate("stat_format")}</div><div class="stat-v">${(status && status.image_format || "png").toUpperCase()}</div></div>
      </div>
      <div class="hero-actions">
        <button class="btn btn-secondary" disabled=${busy} onClick=${doShot}><${Icon} name="camera" size=${17} />${translate("btn_shot")}</button>
        ${rec
          ? html`<button class="btn btn-destructive" disabled=${busy} onClick=${() => doRecord("stop")}><${Icon} name="square" size=${16} />${translate("btn_stop")}</button>`
          : html`<button class="btn btn-primary" disabled=${busy} onClick=${() => doRecord("start")}><${Icon} name="circle" size=${16} />${translate("btn_start")}</button>`}
      </div>
    </div>

    <div class="tabs">
      <button class=${"tab " + (tab === "gallery" ? "active" : "")} onClick=${() => setTab("gallery")}>${translate("tab_gallery")}</button>
      <button class=${"tab " + (tab === "settings" ? "active" : "")} onClick=${() => setTab("settings")}>${translate("tab_settings")}</button>
    </div>

    ${tab === "gallery" && html`<${GalleryView} ctx=${galleryContext} />`}


    ${tab === "settings" && (SettingsComponent ? html`<${SettingsComponent} cfg=${cfg} toast=${toast} onLangPreview=${(language) => setCfg((previous) => ({ ...(previous || {}), language }))} onSaved=${() => { refreshConfig(); refreshStatus(); }} />` : html`<div class="empty">…</div>`)}

    ${light >= 0 && LightboxComponent && html`<${LightboxComponent} items=${items} index=${light}
      onClose=${() => setLight(-1)}
      onNav=${(delta) => setLight((index) => Math.max(0, Math.min(items.length - 1, index + delta)))}
      onDelete=${(id) => doDelete([id])} zoom=${zoom} setZoom=${setZoom} toast=${toast} />`}

    ${toastMsg && html`<div class=${"toast " + (toastMsg.err ? "err" : "")}>
      <${Icon} name=${toastMsg.err ? "x" : "check"} size=${16} />${toastMsg.msg}</div>`}
  </div>`;
}

export { App };
