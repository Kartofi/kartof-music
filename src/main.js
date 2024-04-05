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

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsgEl.textContent = await invoke("enqueue", {
    path: greetInputEl.value,
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");

  document
    .querySelector("#greet-form")
    .addEventListener("submit", async (e) => {
      e.preventDefault();
      greet();
      let ddwd = await invoke("resume");
    });
});

window.addEventListener("load", async () => {
  var slider = $("#volume-slider");
  let current_volume = await invoke("get_volume");

  slider.get(0).MaterialSlider.change(current_volume * 100);

  slider.on("input", async (value) => {
    await invoke("set_volume", {
      volume: slider.val() / 100,
    });
  });

  setInterval(async () => {
    /* let ddd = await invoke("get_cover_", { path: "audios/Hentai.mp3" });

    document.getElementById("snimka").src =
      "data:image/png;base64," + btoa(String.fromCharCode.apply(null, ddd));
*/
    let playing = await invoke("get_playing");
    console.log(playing);

    let qwe = await invoke("enqueue", { path: "audios/2D Cutie.mp3" });
    //let ddwd = await invoke("pause");
  }, 1000);
});
