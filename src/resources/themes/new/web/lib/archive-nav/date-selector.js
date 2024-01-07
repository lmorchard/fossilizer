class ArchiveNavDateSelector extends HTMLElement {
  constructor() {
    super();
  }

  connectedCallback() {
    const linkTop = document.head.querySelector("link[rel=top]");
    this.topUrl = new URL(linkTop.getAttribute("href"), window.location);
    this.indexJsonURL = new URL("./index.json", this.topUrl);

    this.addEventListener("change", ev => {
      if (ev.target.classList.contains("date-nav")) {
        this.handleNavigationChange(ev);
      }
    });

    this.fetchIndexJSON();
  }

  async fetchIndexJSON() {
    const resp = await fetch(this.indexJsonURL);
    const indexJson = await resp.json();
    const pages = indexJson.sort((a, b) => a.current.date.localeCompare(b.current.date));

    let previous, next;

    const innerHTML = [`<select class="date-nav">`];
    for (const page of pages) {
      const { date, path, count } = page.current;
      const selected = new URL(path, this.topUrl).toString() === window.location.toString();
      if (selected) {
        ({ previous, next } = page);
      }
      innerHTML.push(`
        <option value="${path}" ${selected ? "selected" : ""}>
          ${date} (${count})
        </option>
      `)
    }
    innerHTML.push(`</select>`);

    /*
    if (previous) {
      innerHTML.unshift(`<a href="${new URL(previous.path, this.topUrl)}" class="previous">Previous</a>`);
    }
    if (next) {
      innerHTML.push(`<a href="${new URL(next.path, this.topUrl)}" class="next">Next</a>`);
    }
    */

    this.innerHTML = innerHTML.join("\n");
  }

  handleNavigationChange(ev) {
    const url = new URL(ev.target.value, this.topUrl);
    window.location.assign(url);
  }
}

customElements.define("archive-nav-date-selector", ArchiveNavDateSelector);