class MediaLightboxContext extends HTMLElement {
  connectedCallback() {
    this.sheet = document.createElement("style");
    this.sheet.type = "text/css";
    this.sheet.innerText = this.constructor.css;
    document.head.appendChild(this.sheet);

    this.lightbox = document.adoptNode(document
      .createRange()
      .createContextualFragment(this.constructor.html).firstElementChild);
    document.body.appendChild(this.lightbox);
  }

  disconnectedCallback() {
    this.sheet.remove();
    this.lightbox.remove();
  }

  show(src, description, previous, next) {
    this.lightbox.show(src, description, previous, next);
  }

  static html = /*html*/ `
    <media-lightbox>
      <section class="main">
        <img src="" />
      </section>
      <div class="description"></div>
      <button class="dismiss visible">𐌢</button>
      <button class="previous">⏴</button>
      <button class="next">⏵</button>
    </media-lightbox>
  `

  static css = /*css*/ `
    media-lightbox {
      display: none;
      position: fixed;
      left: 0;
      top: 0;
      z-index: 10000;
      width: 100vw;
      height: 100vh;
      background: rgba(0, 0, 0, 0.9);
    }
    media-lightbox.visible {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
    }
    media-lightbox section.main img {
      max-width: 80vw;
      max-height: 80vh;
    }
    media-lightbox .description {
      display: none;
      max-width: 50vw;
      margin-top: 3em;
      padding: 1.5em;
      background: rgba(255,255,255,0.125);
    }
    media-lightbox .description.visible {
      display: block;
    }
    media-lightbox button {
      display: none;
      position: absolute;
      font-size: 2.5em;
      cursor: pointer;
      border: none;
      background: none;
      color: #888;
      z-index: 25000;
    }
    media-lightbox button.visible {
      display: block;
    }
    media-lightbox button.dismiss {
      top: 1vh;
      right: 1vw;
    }
    media-lightbox button.previous {
      top: 50vh;
      left: 1vw;
    }
    media-lightbox button.next {
      top: 50vh;
      right: 1vw;
    }
    @media (prefers-color-scheme: light) {
      media-lightbox button.dismiss {
        color: #333;
      }
      media-lightbox section.main img {
        background: rgba(128,128,128,0.2);
      }
      media-lightbox .description {
        background: rgba(128,128,128,0.2);
      }
    }
    @media (prefers-color-scheme: dark) {
      media-lightbox button.dismiss {
        color: #ddd;
      }
      media-lightbox section.main img {
        border: 1px solid rgba(255,255,255,0.2);
      }
      media-lightbox .description {
        background: rgba(255,255,255,0.2);
      }
    }
  `
}

customElements.define("media-lightbox-context", MediaLightboxContext);

class MediaLightbox extends HTMLElement {
  connectedCallback() {
    this.addEventListener(
      "click",
      (ev) => (ev.target === this) && this.dismiss()
    );

    Object.entries({
      "button.dismiss": () => this.dismiss(),
      "button.previous": () => this.showPrevious(),
      "button.next": () => this.showNext(),
    }).forEach(([sel, fn]) => this.querySelector(sel).addEventListener("click", fn));

    this.keyListener = (ev) => this.handleKeyDown(ev);
    document.addEventListener("keyup", this.keyListener);
  }

  disconnectedCallback() {
    document.removeEventListener("keyup", this.keyListener);
  }

  show(src, description, previous, next) {
    const img = this.querySelector("section.main img");
    img.setAttribute("src", src);
    img.setAttribute("title", description);

    const descriptionEl = this.querySelector(".description");
    descriptionEl.innerHTML = description;
    descriptionEl.classList[!!description ? "add" : "remove"]("visible");

    this.previous = previous;
    this.querySelector("button.previous").classList[!!previous ? "add" : "remove"]("visible");

    this.next = next;
    this.querySelector("button.next").classList[!!next ? "add" : "remove"]("visible");

    this.classList.add("visible");
  }

  dismiss() {
    this.classList.remove("visible");
  }

  handleKeyDown(ev) {
    if (!this.classList.contains("visible")) return;

    switch (ev.key) {
      case "Escape":
        return this.dismiss();
      case "ArrowLeft":
        return this.showPrevious();
      case "ArrowRight":
        return this.showNext();
    }
  }

  showPrevious() {
    this.previous && this.previous.show();
  }

  showNext() {
    this.next && this.next.show();
  }
}

customElements.define("media-lightbox", MediaLightbox);

class MediaLightboxList extends HTMLElement { }

customElements.define("media-lightbox-list", MediaLightboxList);

class MediaLightboxItem extends HTMLElement {
  connectedCallback() {
    this.addEventListener("click", ev => this.handleClick(ev));
  }

  handleClick(ev) {
    ev.preventDefault();
    ev.stopPropagation();
    this.show();
  }

  show() {
    const context = this.closest("media-lightbox-context");
    if (context) {
      const link = this.querySelector("a");
      context.show(
        link.href,
        link.title,
        this.previousElementSibling,
        this.nextElementSibling
      );
    }
  }
}

customElements.define("media-lightbox-item", MediaLightboxItem);
