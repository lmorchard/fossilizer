// TODO: vendor a local copy of timeago.js
import { format } from "//unpkg.com/timeago.js@4.0.2/esm/index.js";

// TODO: control this via attribute in formatted-time-context
const ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD = 10000;

class FormattedTimeContext extends HTMLElement {
  connectedCallback() {
    this.updateTimer = setInterval(
      () => this.updateAll(),
      ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD
    );
  }
  disconnectedCallback() {
    if (this.updateTimer) {
      clearInterval(this.updateTimer);
    }
  }
  toggleRelativeTime() {
    // TODO: switch to an attribute
    this.classList.toggle("relative-time");
    this.updateAll();
  }
  shouldUseRelativeTime() {
    return this.classList.contains("relative-time");
  }
  updateAll() {
    for (const el of this.querySelectorAll("[is=formatted-time]")) {
      el.update();
    }
  }
}

customElements.define("formatted-time-context", FormattedTimeContext);

class FormattedTime extends HTMLTimeElement {
  connectedCallback() {
    this.update();
  }
  update() {
    // TODO: maybe also control this with a local attribute?
    let timeSince = false;
    const parent = this.closest("formatted-time-context");
    if (parent && parent.shouldUseRelativeTime()) {
      timeSince = true;
    }
    const datetime = new Date(this.getAttribute("datetime"));
    this.innerHTML = timeSince ? format(datetime) : datetime.toLocaleString();
  }
}

customElements.define("formatted-time", FormattedTime, { extends: "time" });
