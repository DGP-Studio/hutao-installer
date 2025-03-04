<template>
  <div class="main">
    <div v-show="!init" class="init-loading">
      <span class="fui-Spinner__spinner">
        <span class="fui-Spinner__spinnerTail"></span>
      </span>
    </div>
    <div v-show="init" class="content">
      <div class="image">
        <img src="./hutao.png" />
      </div>
      <div class="right">
        <div class="title">Snap Hutao</div>
        <div class="desc">实用的开源多功能原神工具箱 🧰</div>
        <div v-if="step === 1" class="actions">
          <div v-if="!CONFIG.is_update" class="lnk">
            <Checkbox v-model="createLnk" />
            创建桌面快捷方式
          </div>
          <div v-if="!CONFIG.is_update" class="read">
            <Checkbox v-model="acceptEula" />
            我已阅读并同意
            <a @click="openTos"> 用户协议 </a>
          </div>
          <button class="btn btn-install" @click="start" :disabled="!CONFIG.is_update && !acceptEula">
            开始
          </button>
        </div>
        <div class="login" v-if="step === 2">
          <div class="desc">如果你购买了胡桃云 CDN 服务，你可以在这里登录以获取更好的下载体验</div>
          <textarea class="textarea" v-model="homaUsername" placeholder="用户名"></textarea>
          <textarea class="textarea textarea-password" v-model="homaPassword" placeholder="密码"></textarea>
          <div class="btn-container">
            <button class="btn btn-login" @click="loginSkip">跳过</button>
            <button class="btn btn-login" @click="login"
              :disabled="!emailRegex.test(homaUsername) || homaPassword.length === 0 || logging">
              <div v-if="!logging">登录</div>
              <span v-if="logging" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail"></span>
              </span>
            </button>
          </div>
        </div>
        <div class="choose-mirror" v-if="step === 3">
          <div class="choose-mirror-desc">
            <div class="desc">选择一个镜像源</div>
            <div class="listview">
              <div v-for="(item, index) in mirrors" :key="index" class="listview-item"
                :class="{ selected: selectedMirror === item }" @click="onItemClick(item)">
                <div class="left-indicator"></div>
                <div class="mirror-item">
                  <span>{{ item.mirror_name }}</span>
                  <span>{{ item.speed == -1 ? "timeout" : `${item.speed?.toFixed(2)} MB/s` }}</span>
                </div>
              </div>
            </div>
          </div>
          <button class="btn btn-install" @click="install" :disabled="!selectedMirror">
            {{ CONFIG.is_update ? '更新' : '安装' }}
          </button>
        </div>
        <div class="progress" v-if="step === 4">
          <div class="step-desc">
            <div v-for="(i, a) in subStepList" class="substep" :class="{ done: a < subStep }" v-show="a <= subStep"
              :key="i">
              <span v-if="a === subStep" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail"></span>
              </span>
              <span v-else class="substep-done">
                <CircleSuccess />
              </span>
              <div>{{ i }}</div>
            </div>
          </div>
          <div class="current-status" v-html="current"></div>
          <div class="progress-bar" :style="{ width: `${percent}%` }"></div>
        </div>
        <div class="finish" v-if="step === 5">
          <div class="finish-text">
            <CircleSuccess />
            {{ CONFIG.is_update ? '更新' : '安装' }}完成
          </div>
          <button class="btn btn-install" @click="launch">启动</button>
        </div>
        <div class="finish" v-if="step === 6">
          <div class="finish-text">
            <CircleSuccess />
            您已安装最新版本
          </div>
          <button class="btn btn-install" @click="launch">启动</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.main {
  min-height: 100vh;
}

.init-loading {
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  padding-bottom: 24px;
  box-sizing: border-box;
}

.init-loading .fui-Spinner__spinner {
  width: 40px;
  height: 40px;
  --fui-Spinner--strokeWidth: 4px;
}

.content {
  display: flex;
  min-height: 100vh;
  line-height: 1.1;
  text-align: center;
  justify-content: center;
  user-select: none;
  padding: 0 16px;
  gap: 8px;
}

.desc {
  font-size: 14px;
  opacity: 0.8;
  padding-left: 10px;
  padding-bottom: 2px;
  line-height: 1.4;
}

.textarea {
  width: 100%;
  height: 32px;
  padding: 6px;
  border-radius: 4px;
  box-sizing: border-box;
  font-size: 12px;
  resize: none;
  opacity: 0.8;
  margin-left: 10px;
  margin-top: 4px;
  font-family:
    Consolas,
    'Courier New',
    Microsoft Yahei;
}

.textarea-password {
  -webkit-text-security: disc;
}

.image {
  min-width: 200px;
  width: 200px;
  box-sizing: border-box;
  padding: 6px 6px 6px 6px;

  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
}

.right {
  position: relative;
  width: calc(100% - 188px);
  text-align: left;
  display: flex;
  flex-direction: column;
  padding: 16px;
  box-sizing: border-box;
  overflow: hidden;
}

.title {
  font-size: 25px;
  padding: 2px 10px 6px;
}

.btn-container {
  display: flex;
  position: absolute;
  height: 40px;
  width: 248px;
  margin-left: 10px;
  bottom: 20px;
  gap: 10px;

  .fui-Spinner__spinner {
    width: 16px;
    height: 16px;
    display: block;
  }
}

.btn-login {
  height: 40px;
  width: 140px;
  bottom: 20px;
  right: 8px;
}

.btn-install {
  height: 40px;
  width: 140px;
  position: absolute;
  bottom: 20px;
  right: 8px;
}

.actions,
.login {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 8px;
}

.read,
.lnk {
  align-items: center;
  gap: 4px;
  padding-left: 12px;
  font-size: 13px;
  display: flex;

  a {
    cursor: pointer;
  }
}

.more {
  align-items: flex-start;
  gap: 6px;
  padding-top: 8px;
  padding-left: 10px;
  font-size: 13px;
  display: flex;
  flex-direction: column;

  span {
    opacity: 0.8;
  }

  a {
    cursor: pointer;
    font-family:
      Consolas,
      'Courier New',
      Microsoft Yahei;
    opacity: 0.8;
    font-size: 12px;
  }
}

.finish-text {
  text-align: center;
  opacity: 0.9;
  width: 100%;
  margin-top: 20px;
  padding: 38px 10px;
  font-size: 18px;
  display: flex;
  justify-content: center;
  gap: 8px;
  align-items: center;

  svg {
    width: 24px;
  }
}

.progress-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  height: 4px;
  background: var(--colorBrandForeground1);
  transition: width 0.1s;
  transition-timing-function: cubic-bezier(0.33, 0, 0.67, 1);
  /* easeInOut */
  width: 30%;
}

.choose-mirror-desc {
  padding: 14px 0px;
  font-size: 14px;
  display: flex;
  flex-direction: column;
}

.step-desc {
  padding: 14px 10px;
  font-size: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.substep {
  display: flex;
  gap: 6px;

  .fui-Spinner__spinner {
    width: 16px;
    height: 16px;
    display: block;
  }

  .substep-done {
    width: 16px;
    height: 16px;
    display: block;
  }
}

.substep.done {
  font-size: 13px;
  opacity: 0.8;
}

.current-status {
  position: relative;
  max-width: 100%;
  font-size: 12px;
  opacity: 0.7;
  padding-left: 14px;
  margin-top: -6px;
  font-family:
    Consolas,
    'Courier New',
    Microsoft Yahei;
}

.listview {
  max-height: 400px;
  overflow-y: auto;
  padding: 4px;
  gap: 4px;
}

.listview-item {
  display: flex;
  align-items: center;
  padding: 10px;
  height: 20px;
  cursor: pointer;
  transition: background 0.2s ease;
  border-radius: 4px;
  /* 圆角 */
}

.listview-item:hover {
  background-color: #2d2d2d;
}

.listview-item.selected {
  background-color: #2d2d2d;
}

.left-indicator {
  width: 4px;
  height: 0px;
  opacity: 0;
  background-color: #0f6cbd;
  margin-right: 8px;
  border-radius: 2px;
  transition: height 0.1s ease,
    opacity 0.1s ease;
}

.listview-item.selected .left-indicator {
  height: 16px;
  opacity: 1;
}

.mirror-item {
  display: flex;
  justify-content: space-between;
  width: 100%;
  font-size: 14px;
  gap: 8px;
}
</style>
<style>
.d-single-stat {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.d-single-list {
  display: flex;
  flex-direction: column;
  height: 55px;
  overflow: hidden;
  padding-top: 4px;
  font-size: 11px;
  gap: 2px;
  width: 230px;
  max-height: 250px;
  overflow-y: auto;
  padding-left: 20px;

  &::-webkit-scrollbar {
    width: 4px;
  }

  &::-webkit-scrollbar-thumb {
    background: var(--colorBrandForeground1);
    border-radius: 4px;
  }

  &::-webkit-scrollbar-track {
    background: var(--colorBrandBackground);
  }

  &::-webkit-scrollbar-thumb:hover {
    background: var(--colorBrandForeground2);
  }
}

.d-single {
  display: flex;
  justify-content: space-between;
  gap: 8px;
}

.d-single-filename {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.d-single-progress {
  width: 36px;
  min-width: 36px;
}
</style>

<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { getCurrentWindow, invoke, sep } from './tauri';
import Checkbox from './Checkbox.vue';
import CircleSuccess from './CircleSuccess.vue';
import { fetchPatchData, IsCdnAvailable, LoadToken, LoginHomaPassport } from "./api";

const init = ref(false);

const subStepList: ReadonlyArray<string> = [
  '下载安装包',
  '准备运行环境',
  '部署文件',
];

const acceptEula = ref<boolean>(true);
const createLnk = ref<boolean>(true);
/**
 * 1: EULA
 * 2: Login
 * 3: Choose Mirror
 * 4: Downloading
 * 5: Finish
 * 6: Already Installed
 */
const step = ref<number>(1);
const subStep = ref<number>(0);

const current = ref<string>('');
const percent = ref<number>(0);
const homaUsername = ref<string>('');
const homaPassword = ref<string>('');
const progressInterval = ref<number>(0);

const mirrors = ref<GenericPatchPackageMirror[]>([]);
const selectedMirror = ref<GenericPatchPackageMirror | null>(null);
const isCdnAvailable = ref<boolean>(false);
const logging = ref<boolean>(false);

const CONFIG: Config = reactive({
  is_update: false,
  curr_version: null,
  token: null,
});

const emailRegex = /^[\w-]+(\.[\w-]+)*@[\w-]+(\.[\w-]+)+$/;

async function openTos(): Promise<void> {
  await invoke('open_tos');
}

async function start(): Promise<void> {
  if (CONFIG.token) {
    LoadToken(CONFIG.token);
    if (await IsCdnAvailable()) {
      isCdnAvailable.value = true;
      step.value = 4;
    } else {
      step.value = 3;
    }
    return;
  }

  step.value = 2;
}

async function login(): Promise<void> {
  logging.value = true;
  if (!await LoginHomaPassport(homaUsername.value, homaPassword.value)) {
    logging.value = false;
    return;
  }
  if (await IsCdnAvailable()) {
    isCdnAvailable.value = true;
    step.value = 4;
  } else {
    await invoke('message_dialog', {
      title: '无 CDN 权限',
      message: '未检测到有效 CDN 权限，请选择一个镜像源进行下载安装包',
    })
    step.value = 3;
  }
  logging.value = false;
}

async function loginSkip(): Promise<void> {
  step.value = 3;
}

async function install(): Promise<void> {

}

async function launch(): Promise<void> {
  await invoke('launch_and_exit');

}

function onItemClick(item: GenericPatchPackageMirror): void {
  selectedMirror.value = item;
}

async function testMirrorSpeed(): Promise<void> {
  const testers = [];
  for (const mirror of mirrors.value) {
    testers.push(invoke<number>('speedtest_1mb', { url: mirror.url }).then(s => mirror.speed = s));
  }

  await Promise.all(testers);
  mirrors.value = mirrors.value.sort((a, b) => (b.speed ?? -1) - (a.speed ?? -1));
}

onMounted(async () => {
  const win = getCurrentWindow();
  await win.setTitle('Snap Hutao Deployment');
  await win.show();

  let config = await invoke<Config>('get_config');
  Object.assign(CONFIG, config);
  let patch_data = await fetchPatchData();
  mirrors.value = patch_data.mirrors;

  if (config.is_update && config.curr_version) {
    let local = Version.parse(config.curr_version);
    let remote = Version.parse(patch_data.version);
    if (true || remote.compare(local) <= 0) {
      step.value = 6;
      init.value = true;
      return;
    }
  }

  testMirrorSpeed();
  init.value = true;
})

class Version {
  major: number;
  minor: number;
  build: number;
  revision: number;

  constructor(
    major: number,
    minor: number,
    build: number | undefined,
    revision: number | undefined
  ) {
    this.major = major;
    this.minor = minor;
    this.build = build ?? 0;
    this.revision = revision ?? 0;
  }

  toString() {
    return `${this.major}.${this.minor}.${this.build}.${this.revision}`;
  }

  static parse(version: string) {
    const [major, minor, build, revision] = version.split(".").map(Number);
    return new Version(major, minor, build, revision);
  }

  compare(other: Version) {
    if (this.major !== other.major) {
      return this.major - other.major;
    }
    if (this.minor !== other.minor) {
      return this.minor - other.minor;
    }
    if (this.build !== other.build) {
      return this.build - other.build;
    }
    return this.revision - other.revision;
  }
}


</script>
