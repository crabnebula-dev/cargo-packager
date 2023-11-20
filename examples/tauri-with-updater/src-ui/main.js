import {
  createApp,
  ref,
  onMounted,
  onUnmounted,
  computed,
} from "https://unpkg.com/vue@3/dist/vue.esm-browser.prod.js";
const { invoke } = window.__TAURI__;
const { listen } = window.__TAURI__.event;

createApp({
  setup() {
    const version = ref("");
    const updateStatus = ref("unchecked"); // variants: "unchecked" | "has-update" | "no-updates" | "downloading" | "ready-for-install" | "installing"
    const updateContentLen = ref(0);
    const updateDownloadedDataLen = ref(0);
    const updateProgress = ref(0);
    const updateVersion = ref("");
    const checkBtnDisabled = computed(
      () =>
        updateStatus.value === "has-update" ||
        updateStatus.value == "downloading" ||
        updateStatus.value === "ready-for-install"
    );

    async function checkUpdate() {
      const [hasUpdate, version] = await invoke("check_update");
      if (version) updateVersion.value = version;
      updateStatus.value = hasUpdate ? "has-update" : "no-updates";
    }

    async function downloadUpdate() {
      updateStatus.value = "downloading";
      await invoke("download_update");
      updateStatus.value = "ready-for-install";
    }

    async function installUpdate() {
      await invoke("install_update");
    }

    let removeProgressListener;

    onMounted(async () => {
      version.value = await invoke("version");

      removeProgressListener = await listen("update_progress", (event) => {
        const { chunk_len, content_len } = event.payload;
        if (content_len) {
          updateContentLen.value = content_len;
        }

        updateDownloadedDataLen.value =
          updateDownloadedDataLen.value + chunk_len;

        updateProgress.value =
          (updateDownloadedDataLen.value / updateContentLen.value) * 100;
      });
    });

    onUnmounted(() => removeProgressListener());

    return {
      version,
      updateStatus,
      updateVersion,
      updateContentLen,
      updateDownloadedDataLen,
      updateProgress,
      checkBtnDisabled,
      checkUpdate,
      downloadUpdate,
      installUpdate,
    };
  },
  template: `
    <p>appVersion: {{version}}</p>
    <button @click="checkUpdate" :disabled="checkBtnDisabled">Check Update</button>

    <template v-if="updateStatus !== 'unchecked'">
      <p v-if="updateStatus === 'no-updates'">There is no updates available!</p>
      <p v-else>There is a new version available! <span v-if="!!updateVersion">({{updateVersion}})</span></p>

      <button
        v-if="updateStatus === 'has-update' || updateStatus === 'downloading' || updateStatus === 'ready-for-install'"
        :disabled="updateStatus === 'downloading' || updateStatus === 'ready-for-install'"
        @click="downloadUpdate"
      >
        Download
      </button>
      <p v-if="updateStatus === 'downloading' || updateStatus === 'ready-for-install'">{{updateProgress.toFixed(1)}}</p>

      <button v-if="updateStatus === 'ready-for-install'" @click="installUpdate">Install</button>
      </template>
  `,
}).mount("#app");
