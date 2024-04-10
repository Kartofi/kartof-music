const { invoke } = window.__TAURI__.tauri;

let greetInputEl;
let greetMsgEl;

const { appWindow } = window.__TAURI__.window;
document
  .getElementById("titlebar-minimize")
  .addEventListener("click", () => appWindow.minimize());
document
  .getElementById("titlebar-maximize")
  .addEventListener("click", () => appWindow.toggleMaximize());
document
  .getElementById("titlebar-close")
  .addEventListener("click", () => appWindow.close());

window.addEventListener("load", async () => {
  var slider = $("#volume-slider");
  let current_volume = await invoke("get_volume");

  slider.get(0).MaterialSlider.change(current_volume * 100);

  slider.on("input", async (value) => {
    await invoke("set_volume", {
      volume: slider.val() / 100,
    });
  });
});
