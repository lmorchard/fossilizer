// TODO: control this via attribute in formatted-time-context
const ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD = 10000;

class FormattedTimeContext extends HTMLElement {
  connectedCallback() {
    this.updateTimer = setInterval(
      () => this.setAttribute("last-update", Date.now()),
      ARCHIVE_ACTIVITY_TIME_UPDATE_PERIOD
    );
  }
  disconnectedCallback() {
    if (this.updateTimer) clearInterval(this.updateTimer);
  }
  toggleRelativeTime() {
    // TODO: switch to an attribute
    this.classList.toggle("relative-time");
  }
  setRelativeTime(value) {
    this.classList[value ? "add" : "remove"]("relative-time");
  }
  shouldUseRelativeTime() {
    return this.classList.contains("relative-time");
  }
}

customElements.define("formatted-time-context", FormattedTimeContext);

class FormattedTime extends HTMLTimeElement {
  connectedCallback() {
    this.update();

    this.context = this.closest("formatted-time-context");
    if (this.context) {
      this.contextObserver = new MutationObserver(() => this.update());
      this.contextObserver.observe(
        this.context,
        { attributeFilter: ["class", "last-update"] }
      )
    }
  }
  disconnectedCallback() {
    this.contextObserver.disconnect();
  }
  update() {
    // TODO: maybe also control this with a local attribute?
    let timeSince = false;
    if (this.context && this.context.shouldUseRelativeTime()) {
      timeSince = true;
    }

    const datetime = new Date(this.getAttribute("datetime"));
    if (timeSince && timeago && timeago.format) {
      this.innerHTML = timeago.format(datetime);
    } else {
      this.innerHTML = datetime.toLocaleString();
    }
  }
}

customElements.define("formatted-time", FormattedTime, { extends: "time" });
