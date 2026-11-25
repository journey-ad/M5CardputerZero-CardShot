import { html } from "../core.js";

export function GalleryView({ ctx }) {
  const {
    items, pageItems, totalPages, page, pageSize, view, sel, selIds, marquee,
    galleryRef, gridRef, translate, Icon, Thumb, humanSize, mmss, fmtDate,
    setView, setPageSize, setCurPage, selectAll, enterSelect, doDelete, setSel, pickItem,
  } = ctx;
  return html`<div class="gallery-wrap" ref=${galleryRef}>
      <div class="toolbar">
        <div class="count">${translate("total_pre")}${items.length}${translate("total_suf")}</div>
        <div class="spacer"></div>
        <div class="view-toggle">
          <button class=${view === "grid" ? "active" : ""} title=${translate("title_grid")} onClick=${() => setView("grid")}><${Icon} name="grid" size=${16} /></button>
          <button class=${view === "list" ? "active" : ""} title=${translate("title_list")} onClick=${() => setView("list")}><${Icon} name="list" size=${16} /></button>
        </div>
        ${sel.mode
          ? html`<span class="count">${translate("sel_pre")}${selIds.length}</span>
              <button class="btn btn-outline btn-sm" onClick=${selectAll}>${translate("select_all")}</button>
              <button class="btn btn-destructive btn-sm" disabled=${!selIds.length} onClick=${() => doDelete(selIds)}><${Icon} name="trash-2" size=${15} />${translate("delete")}</button>
              <button class="btn btn-ghost btn-sm" onClick=${() => setSel({ mode: false, ids: new Set() })}>${translate("cancel")}</button>`
          : html`<button class="btn btn-outline btn-sm" disabled=${!items.length} onClick=${() => enterSelect(null)}><${Icon} name="check" size=${15} />${translate("multi")}</button>`}
        <select class="select page-size" value=${pageSize} onChange=${(event) => { setPageSize(Number(event.currentTarget.value)); setCurPage(1); }}>
          ${[12, 24, 48, 96].map((count) => html`<option value=${count} selected=${pageSize === count}>${translate("per_page")} ${count}</option>`)}
        </select>
        <button class="btn btn-ghost btn-icon" disabled=${page <= 1} onClick=${() => setCurPage(page - 1)} title=${translate("prev")}><${Icon} name="chevron-left" size=${18} /></button>
        <span class="count mono">${page} / ${totalPages}</span>
        <button class="btn btn-ghost btn-icon" disabled=${page >= totalPages} onClick=${() => setCurPage(page + 1)} title=${translate("next")}><${Icon} name="chevron-right" size=${18} /></button>
      </div>
      ${items.length === 0
        ? html`<div class="empty"><${Icon} name="image" size=${40} /><div>${translate("empty")}</div></div>`
        : view === "grid"
        ? html`<div class="grid" ref=${gridRef}>
            ${pageItems.map((it, idx) => html`<div data-tid=${it.id} class=${"tile " + (sel.mode ? "selecting " : "") + (sel.ids.has(it.id) ? "selected" : "")}
              onClick=${() => pickItem(it, idx)}>
              <div class="thumb">
                <${Thumb} id=${it.id} kind=${it.kind} />
                ${it.kind === "video" && html`<div class="play"><span><${Icon} name="play" size=${18} /></span></div>`}
                <div class="kind-tag"><${Icon} name=${it.kind === "video" ? "film" : "image"} size=${11} />${it.kind === "video" ? translate("kind_video") : translate("kind_shot")}</div>
                ${it.kind === "video" && it.duration ? html`<div class="dur">${mmss(it.duration)}</div>` : null}
                <div class="check"><${Icon} name="check" size=${15} /></div>
              </div>
              <div class="tile-meta">
                <div class="tile-name">${it.name}</div>
                <div class="tile-sub"><span>${humanSize(it.size)}</span><span>${fmtDate(it.mtime)}</span></div>
              </div>
            </div>`)}
          </div>`
        : html`<div class="list" ref=${gridRef}>
            ${pageItems.map((it, idx) => html`<div data-tid=${it.id} class=${"list-row " + (sel.mode ? "selecting " : "") + (sel.ids.has(it.id) ? "selected" : "")}
              onClick=${() => pickItem(it, idx)}>
              <div class="list-thumb">
                <${Thumb} id=${it.id} kind=${it.kind} />
                ${it.kind === "video" && it.duration ? html`<div class="dur">${mmss(it.duration)}</div>` : null}
              </div>
              <div class="list-info">
                <div class="list-name">${it.name}</div>
                <div class="list-sub"><span>${it.kind === "video" ? translate("kind_video") : translate("kind_shot")}</span><span>${humanSize(it.size)}</span><span>${fmtDate(it.mtime)}</span></div>
              </div>
              <div class="list-check"><${Icon} name="check" size=${14} /></div>
            </div>`)}
          </div>`}
      ${marquee && html`<div class="marquee" style=${`left:${marquee.x}px;top:${marquee.y}px;width:${marquee.width}px;height:${marquee.height}px`}></div>`}
    </div>`;
}
