import { WebSocketHandler } from "./websockets.js";

const KEY_HOME_INSTANCE_DOMAIN = "home-instance-domain";

function init() {
  const searchField = document.body.querySelector("#searchField");
  if (searchField) {
    searchField.focus();
    searchField.select();
  }
  document.body.addEventListener("click", handleClick);
  document.body.addEventListener("change", handleChange);

  setupWebSocket();
}

function setupWebSocket() {
  new WebSocketHandler().connect();
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

async function handleChange(ev) {
  const { classList } = ev.target;
  if (classList.contains("autosubmit")) {
    let current = ev.target;
    do {
      current = current.parentNode;
      if (current.tagName == "FORM") {
        current.submit();
        break;
      }
    } while (current);
  }
}

init();
