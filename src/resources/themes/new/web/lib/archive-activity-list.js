class ArchiveActivityList extends HTMLElement { }

customElements.define("archive-activity-list", ArchiveActivityList);

class ArchiveActivityListContents extends HTMLElement { }

customElements.define("archive-activity-list-contents", ArchiveActivityListContents);

class ArchiveActivityListControls extends HTMLElement {
  connectedCallback() {
    this.querySelector("input[name=grid]")
      .addEventListener("change", (ev) => this.handleChangeGrid(ev));
    this.querySelector("input[name=relative-time]")
      .addEventListener("change", (ev) => this.handleChangeRelativeTime(ev));
  }

  handleChangeGrid(ev) {
    this.getList().classList[ev.target.checked ? "add" : "remove"]("grid");
  }

  handleChangeRelativeTime(ev) {
    document.querySelector("formatted-time-context")
      .setRelativeTime(ev.target.checked);
  }

  getList() {
    const forId = this.getAttribute("for");
    return forId
      ? document.body.querySelector(`#${forId}`)
      : this.closest("archive-activity-list");
  }
}

customElements.define("archive-activity-list-controls", ArchiveActivityListControls);

class ArchiveActivity extends HTMLElement {
  connectedCallback() {
    const hash = window.location.hash;
    if (hash === `#anchor-${this.id}`) {
      this.classList.add("highlighted");
    }
  }
}

customElements.define("archive-activity", ArchiveActivity);

class ArchiveActivityListNextPage extends HTMLElement {
  connectedCallback() {
    this.addEventListener("click", (ev) => this.handleClick(ev));
  }

  async handleClick(ev) {
    if (ev.target.tagName !== "A") return;

    ev.preventDefault();
    ev.stopPropagation();

    // Get the nav link href and then remove the link from the DOM
    const target = ev.target;
    const href = target.getAttribute("href");
    this.removeChild(target);

    // Fetch the nav link href and parse into a document
    const response = await fetch(href);
    const content = await response.text();
    const parser = new DOMParser();
    const doc = parser.parseFromString(content, "text/html");
    const body = doc.body;

    // Find the archive-activity nodes in the loaded document, adopt them into the current page
    const parentList = this.closest("archive-activity-list");
    const container = parentList.querySelector("archive-activity-list-contents");
    for (const node of body.querySelectorAll("archive-activity")) {
      container.appendChild(document.adoptNode(node));
    }

    // Find a next link from the loaded document, adopt it into the current page if found
    const newNextPageLink = body.querySelector("archive-activity-list-next-page a");
    if (newNextPageLink) {
      this.appendChild(document.adoptNode(newNextPageLink));
    }
  }
}

customElements.define("archive-activity-list-next-page", ArchiveActivityListNextPage);
