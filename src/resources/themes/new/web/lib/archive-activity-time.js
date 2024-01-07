// TODO: vendor a local copy of timeago.js
import { format } from "//unpkg.com/timeago.js@4.0.2/esm/index.js";

const ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD = 10000;

class ArchiveActivityTime extends HTMLElement {
  connectedCallback() {
    this.update();
  }
  update() {
    // TODO: maybe also control this with an attribute?
    let timeSince = false;
    const parent = this.closest("archive-activity-list");
    if (parent) {
      if (parent.classList.contains("time-since")) {
        timeSince = true;
      }
    }

    const datetime = new Date(this.getAttribute("datetime"));
    this.innerHTML = timeSince ? format(datetime) : datetime.toLocaleString();
  }
}

customElements.define("archive-activity-time", ArchiveActivityTime);

export function updateArchiveActivityTimeElements() {
  for (const el of document.querySelectorAll("archive-activity-time")) {
    el.update();
  }
}

setInterval(
  () => updateArchiveActivityTimeElements(),
  ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD
);
