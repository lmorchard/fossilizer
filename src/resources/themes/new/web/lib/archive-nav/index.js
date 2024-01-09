import "./date-selector.js";

class ArchiveNav extends HTMLElement {
  constructor() {
    super();
  }

  connectedCallback() {
  }
}

customElements.define("archive-nav", ArchiveNav);

class ArchiveNavSearch extends HTMLElement {
  constructor() {
    super();
  }
  connectedCallback() {
    const linkTop = document.head.querySelector("link[rel=base]");
    const topUrl = new URL(linkTop.getAttribute("href"), window.location);
    const id = `archive-nav-search-${Date.now()}-${Math.ceil(1000 * Math.random())}`;
    this.setAttribute("id", id);

    if (PagefindUI) {
      new PagefindUI({
        element: `#${this.id}`,
        showImages: true,
        showSubResults: true,
        highlightParam: "highlight",
        pageSize: 3,
        baseUrl: topUrl
      });
    }
  }
}

customElements.define("archive-nav-search", ArchiveNavSearch);
