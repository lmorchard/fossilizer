// adapted from https://stackoverflow.com/questions/56300132/how-to-override-css-prefers-color-scheme-setting/75124760#75124760
class ThemeSelector extends HTMLElement {

  connectedCallback() {
    for (const el of this.querySelectorAll(".icon")) {
      el.style.display = "none";
    }
    
    const button = this.querySelector("button");
    if (button) {
      button.addEventListener("click", () => this.toggleColorScheme());
    }

    const scheme = this.getPreferredColorScheme();
    this.applyPreferredColorScheme(scheme);
  }

  toggleColorScheme() {
    let newScheme = "light";
    let scheme = this.getPreferredColorScheme();
    if (scheme === "light") newScheme = "dark";
    this.applyPreferredColorScheme(newScheme);
    this.savePreferredColorScheme(newScheme);
  }

  getPreferredColorScheme() {
    let systemScheme = 'light';
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      systemScheme = 'dark';
    }
    let chosenScheme = systemScheme;
    if (localStorage.getItem("scheme")) {
      chosenScheme = localStorage.getItem("scheme");
    }
    if (systemScheme === chosenScheme) {
      localStorage.removeItem("scheme");
    }
    return chosenScheme;
  }

  savePreferredColorScheme(scheme) {
    let systemScheme = 'light';
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      systemScheme = 'dark';
    }
    if (systemScheme === scheme) {
      localStorage.removeItem("scheme");
    }
    else {
      localStorage.setItem("scheme", scheme);
    }
  }

  applyPreferredColorScheme(scheme) {
    for (let s = 0; s < document.styleSheets.length; s++) {
      for (let i = 0; i < document.styleSheets[s].cssRules.length; i++) {
        const rule = document.styleSheets[s].cssRules[i];
        if (rule && rule.media && rule.media.mediaText.includes("prefers-color-scheme")) {
          switch (scheme) {
            case "light":
              rule.media.appendMedium("original-prefers-color-scheme");
              if (rule.media.mediaText.includes("light")) rule.media.deleteMedium("(prefers-color-scheme: light)");
              if (rule.media.mediaText.includes("dark")) rule.media.deleteMedium("(prefers-color-scheme: dark)");
              break;
            case "dark":
              rule.media.appendMedium("(prefers-color-scheme: light)");
              rule.media.appendMedium("(prefers-color-scheme: dark)");
              if (rule.media.mediaText.includes("original")) rule.media.deleteMedium("original-prefers-color-scheme");
              break;
            default:
              rule.media.appendMedium("(prefers-color-scheme: dark)");
              if (rule.media.mediaText.includes("light")) rule.media.deleteMedium("(prefers-color-scheme: light)");
              if (rule.media.mediaText.includes("original")) rule.media.deleteMedium("original-prefers-color-scheme");
              break;
          }
        }
      }
    }

    if (scheme === "dark") {
      this.querySelector(".icon.light").style.display = "inline";
      this.querySelector(".icon.dark").style.display = "none";
    } else {
      this.querySelector(".icon.dark").style.display = "inline";
      this.querySelector(".icon.light").style.display = "none";
    }
  }
}

customElements.define("theme-selector", ThemeSelector);
