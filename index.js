const KEY_HOME_INSTANCE_DOMAIN = "home-instance-domain";
const LAZY_LOAD_THRESHOLD = 0.1;

function init() {
  document.body.addEventListener("click", handleClick);
  initLazyLoadObserver();
  pageUpdated();
}

function pageUpdated() {
  updateLazyLoadObserver();
}

let lazyLoadObserver;
function initLazyLoadObserver() {
  lazyLoadObserver = new IntersectionObserver(handleAllLazyLoadIntersections, {
    threshold: LAZY_LOAD_THRESHOLD,
  });
}

function updateLazyLoadObserver() {
  lazyLoadObserver.disconnect();
  const toObserve = document.querySelectorAll(".lazy-load");
  for (const element of toObserve) {
    lazyLoadObserver.observe(element);
  }
}

function handleAllLazyLoadIntersections(entries) {
  for (const entry of entries) {
    if (entry.isIntersecting) {
      handleLazyLoadIntersection(entry);
    }
  }
}

async function handleLazyLoadIntersection({ target }) {
  if (/img/i.test(target.tagName)) {
    const src = target.getAttribute("data-src");
    if (src) {
      target.setAttribute("src", src);
      target.removeAttribute("data-src");
    }
  }

  if (target.classList.contains("load-href")) {
    await replaceElementWithHTMLResource(
      target.parentNode,
      target.getAttribute("href")
    );
  }

  if (target.classList.contains("auto-click")) {
    target.classList.remove("auto-click");
    target.click();
  }
}

async function getHomeInstanceDomain() {
  const value = window.localStorage.getItem(KEY_HOME_INSTANCE_DOMAIN);
  if (value) return value;

  const inputValue = window.prompt("Home instance domain?");
  window.localStorage.setItem(KEY_HOME_INSTANCE_DOMAIN, inputValue);
  return inputValue;
}

function resetHomeInstanceDomain() {
  window.localStorage.removeItem(KEY_HOME_INSTANCE_DOMAIN);
  getHomeInstanceDomain();
}

async function handleClick(ev) {
  const { classList } = ev.target;

  if (ev.target.closest(".nav-activities")) {
    return handleNavActivities(ev);
  }

  if (classList.contains("media-attachment")) {
    const { fullsrc } = ev.target.dataset;
    const description = ev.target.getAttribute("title");

    const displayEl = document.querySelector("#media-attachment-display");
    displayEl.src = fullsrc;
    displayEl.setAttribute("alt", description);
    displayEl.setAttribute("title", description);

    const descriptionEl = document.querySelector(
      "#media-attachment-description"
    );
    descriptionEl.innerHTML = ev.target.getAttribute("title");
  }

  if (classList.contains("goto-home-instance")) {
    const { uri } = ev.target.dataset;

    const instanceDomain = await getHomeInstanceDomain();
    const instanceUrl = new URL(
      `https://${instanceDomain}/authorize_interaction`
    );
    instanceUrl.searchParams.set("uri", uri);
    window.open(instanceUrl, "_blank");
  }

  if (classList.contains("reset-home-instance-domain")) {
    resetHomeInstanceDomain();
  }
}

async function handleNavActivities(ev) {
  ev.preventDefault();
  ev.stopPropagation();

  const target = ev.target;
  const link = target.closest(".nav-activities");
  const parent = link.closest(".card");
  const container = parent.closest(".activities");
  const href = link.getAttribute("href");

  const response = await fetch(href);
  const content = await response.text();

  const parser = new DOMParser();
  const doc = parser.parseFromString(content, "text/html");
  const loadedNodes = Array.from(doc.body.querySelector(".activities").children);
  loadedNodes.shift();

  for (const node of loadedNodes) {
    container.insertBefore(document.adoptNode(node), parent);
  }

  parent.remove();

  pageUpdated();
}

init();
