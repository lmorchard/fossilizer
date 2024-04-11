const LAZY_LOAD_THRESHOLD = 0.1;
const LAZY_LOAD_CLASS_NAME = "lazy-load";

class LazyLoadObserver extends HTMLElement {
  constructor() {
    super();

    // TODO: use an attribute for these
    this.threshold = LAZY_LOAD_THRESHOLD;
    this.lazyLoadClassName = LAZY_LOAD_CLASS_NAME;

    this.intersectionObserver = new IntersectionObserver(
      entries => this.handleIntersections(entries),
      { threshold: this.threshold }
    );

    this.mutationObserver = new MutationObserver(
      (records) => this.handleMutations(records)
    );
  }

  connectedCallback() {
    this.mutationObserver.observe(this, {
      subtree: true,
      childList: true,
    });
    for (const node of this.querySelectorAll(`.${this.lazyLoadClassName}`)) {
      this.intersectionObserver.observe(node);
    }
  }

  disconnectedCallback() {
    this.intersectionObserver.disconnect();
    this.mutationObserver.disconnect();
  }

  handleMutations(records) {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType === Node.ELEMENT_NODE) {
          for (const subnode of node.querySelectorAll(`.${this.lazyLoadClassName}`)) {
            this.intersectionObserver.observe(subnode);
          }
          if (node.classList.contains(this.lazyLoadClassName)) {
            this.intersectionObserver.observe(node);
          }
        }
      }
      for (const node of record.removedNodes) {
        if (node.nodeType === Node.ELEMENT_NODE) {
          for (const subnode of node.querySelectorAll(`.${this.lazyLoadClassName}`)) {
            this.intersectionObserver.unobserve(subnode);
          }
          if (node.classList.contains(this.lazyLoadClassName)) {
            this.intersectionObserver.unobserve(node);
          }
        }
      }
    }
  }

  handleIntersections(entries) {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        this.handleIntersection(entry);
      }
    }
  }

  async handleIntersection({ target }) {
    if (/img/i.test(target.tagName)) {
      const src = target.getAttribute("data-src");
      if (src) {
        target.setAttribute("src", src);
        target.removeAttribute("data-src");
      }
    }

    if (target.classList.contains("load-href")) {
      await this.replaceElementWithHTMLResource(
        target.parentNode,
        target.getAttribute("href")
      );
    }

    if (target.classList.contains("auto-click")) {
      target.classList.remove("auto-click");
      target.click();
    }
  }

  async replaceElementWithHTMLResource(element, href) {
    if (element.classList.contains("loading")) return;
    element.classList.add("loading");
    element.setAttribute("disabled", true);

    const response = await fetch(href);
    const content = await response.text();

    const parser = new DOMParser();
    const doc = parser.parseFromString(content, "text/html");
    const loadedNodes = Array.from(doc.body.children);

    const parent = element.parentNode;
    for (const node of loadedNodes) {
      parent.insertBefore(document.adoptNode(node), element);
    }

    element.remove();
  }
}

customElements.define("lazy-load-observer", LazyLoadObserver);
