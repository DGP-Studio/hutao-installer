<template>
  <div class="main">
    <div v-show="!init" class="init-loading">
      <span class="fui-Spinner__spinner">
        <span class="fui-Spinner__spinnerTail"></span>
      </span>
    </div>
    <div v-show="init" class="content">
      <div class="image">
        <img src="/hutao.png" alt="logo" />
      </div>
      <div class="right">
        <div class="title">
          <span>Snap Hutao</span>
        </div>
        <div class="desc">{{ t('实用的开源多功能原神工具箱 🧰') }}</div>
        <div v-if="step === 1" class="actions">
          <div v-if="!CONFIG.is_update" class="lnk">
            <Checkbox v-model="createLnk" />
            <span>{{ t('创建桌面快捷方式') }}</span>
          </div>
          <div v-if="!CONFIG.is_update" class="read">
            <Checkbox v-model="acceptEula" />
            <span>
              {{ t('我已阅读并同意') }}
              <a @click="openTos"> {{ t('用户协议') }} </a>
            </span>
          </div>
          <div v-if="CONFIG.is_update" class="update-info">
            <span>更新信息: {{ version_info }}</span>
            <vue-markdown :source="changelog" class="changelog" />
          </div>
          <button class="btn btn-install" @click="start" :disabled="!CONFIG.is_update && !acceptEula">
            <span>{{ t('开始') }}</span>
          </button>
        </div>
        <div class="login" v-if="step === 2">
          <div class="desc">
            {{
              t(
                '如果你购买了胡桃云 CDN 服务，你可以在这里登录以获取更好的下载体验',
              )
            }}
          </div>
          <input type="email" class="account-input" v-model="homaUsername" :placeholder="t('用户名')" />
          <input type="password" class="account-input textarea-password" v-model="homaPassword"
            :placeholder="t('密码')" />
          <div class="btn-container">
            <button class="btn btn-login" @click="loginSkip">
              {{ t('跳过') }}
            </button>
            <button class="btn btn-login" @click="login" :disabled="!emailRegex.test(homaUsername) ||
              homaPassword.length === 0 ||
              logging_in
              ">
              <span v-if="!logging_in">{{ t('登录') }}</span>
              <span v-if="logging_in" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
            </button>
          </div>
        </div>
        <div class="choose-mirror" v-if="step === 3">
          <div class="choose-mirror-desc">
            <div class="desc">{{ t('选择一个镜像源') }}</div>
            <div class="listview">
              <div v-for="(item, index) in mirrors" :key="index" class="listview-item"
                :class="{ selected: selectedMirror === item }" @click="onItemClick(item)">
                <div class="left-indicator" />
                <div class="mirror-item">
                  <span>{{ item.mirror_name }}</span>
                  <span>
                    {{
                      item.speed == -1
                        ? 'timeout'
                        : `${item.speed?.toFixed(2)} MB/s`
                    }}
                  </span>
                </div>
              </div>
            </div>
          </div>
          <button class="btn btn-install" @click="install" :disabled="!selectedMirror">
            {{ CONFIG.is_update ? t('更新') : t('安装') }}
          </button>
        </div>
        <div class="progress" v-if="step === 4">
          <div class="step-desc">
            <div v-for="(i, a) in subStepList" class="substep" :class="{ done: a < subStep }" v-show="a <= subStep"
              :key="i">
              <span v-if="a === subStep" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else class="substep-done">
                <CircleSuccess />
              </span>
              <div>{{ t(i) }}</div>
            </div>
          </div>
          <div class="current-status" v-html="current" />
          <div class="progress-bar" :style="{ width: `${percent}%` }" />
        </div>
        <div class="finish" v-if="step === 5">
          <div class="finish-text">
            <CircleSuccess />
            <span>{{ CONFIG.is_update ? t('更新完成') : t('安装完成') }}</span>
          </div>
          <button class="btn btn-install" @click="launch">
            {{ t('启动') }}
          </button>
        </div>
        <div class="finish" v-if="step === 6">
          <div class="finish-text">
            <CircleSuccess />
            <span>{{ t('您已安装最新版本') }}</span>
          </div>
          <button class="btn btn-install" @click="launch">
            {{ t('启动') }}
          </button>
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

.account-input {
  width: 100%;
  height: 32px;
  padding: 6px;
  background: var(--colorTextareaBackground);
  color: var(--colorTextareaText);
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
    Microsoft Yahei,
    serif;
  border: unset;
  outline: none;
}

.textarea-password {
  -webkit-text-security: disc;
}

.image {
  min-width: 280px;
  width: 280px;
  box-sizing: border-box;
  padding: 8px;

  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
}

.right {
  position: relative;
  width: calc(100% - 280px);
  text-align: left;
  display: flex;
  flex-direction: column;
  padding: 32px;
  box-sizing: border-box;
  overflow: hidden;
}

.title {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: flex-start;
  font-size: 25px;
  padding: 2px 10px;
  column-gap: 4px;
  line-height: 28px;
}

.btn-container {
  display: flex;
  position: absolute;
  height: 40px;
  width: 266px;
  margin-left: 10px;
  bottom: 36px;
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
}

.btn-install {
  height: 40px;
  width: 140px;
  position: absolute;
  bottom: 36px;
  right: 22px;
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

.update-info {
  padding-left: 12px;
  display: flex;
  line-height: 1.4;
  font-size: 13px;
  gap: 8px;
  flex-direction: column;
}

.changelog {
  overflow-y: auto;
  margin-left: -24px;
  height: 140px;

  ::v-deep(h3) {
    margin-left: 20px;
  }
}

.changelog::-webkit-scrollbar {
  display: none;
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
  padding: 14px 0;
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
  display: flex;
  flex-direction: column;
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

.listview-item:hover,
.listview-item.selected {
  background-color: var(--colorListViewHoverOrSelected);
}

.left-indicator {
  width: 4px;
  height: 0;
  opacity: 0;
  background-color: #0f6cbd;
  margin-right: 8px;
  border-radius: 2px;
  transition:
    height 0.1s ease,
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
</style>

<script setup lang="ts">
import VueMarkdown from 'vue-markdown-render';
import { useI18n } from 'vue-i18n';
import { onMounted, reactive, ref } from 'vue';
import { getCurrentWindow, invoke, listen } from './tauri';
import Checkbox from './components/Checkbox.vue';
import CircleSuccess from './components/CircleSuccess.vue';
import { v4 as uuid } from 'uuid';
import {
  fetchIsOversea,
  fetchPatchData,
  GetCdnUrl,
  IsCdnAvailable,
  LoadToken,
  LoginHomaPassport,
} from './api';
import { getLang } from './i18n';

const { t } = useI18n();
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

const sha256 = ref<string>('');
const mirrors = ref<GenericPatchPackageMirror[]>([]);
const selectedMirror = ref<GenericPatchPackageMirror | null>(null);
const isCdnAvailable = ref<boolean>(false);
const isOversea = ref<boolean>(false);
const logging_in = ref<boolean>(false);
const version_info = ref<string>('');
const changelog = ref<string>('');

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
  if (isOversea.value) {
    selectedMirror.value = mirrors.value[0];
    await install();
    return;
  }

  if (CONFIG.token) {
    await LoadToken(CONFIG.token);
    if (await IsCdnAvailable()) {
      isCdnAvailable.value = true;
      await install();
    } else {
      step.value = 3;
    }
    return;
  }

  step.value = 2;
}

async function login(): Promise<void> {
  logging_in.value = true;
  if (!(await LoginHomaPassport(homaUsername.value, homaPassword.value))) {
    logging_in.value = false;
    return;
  }
  if (await IsCdnAvailable()) {
    isCdnAvailable.value = true;
    await install();
  } else {
    await invoke('message_dialog', {
      title: t('无 CDN 权限'),
      message: t('未检测到有效 CDN 权限，请选择一个镜像源进行下载安装包'),
    });
    step.value = 3;
  }
  logging_in.value = false;
}

async function loginSkip(): Promise<void> {
  step.value = 3;
}

async function install(): Promise<void> {
  step.value = 4;
  current.value = t('准备下载……');
  let mirror_url;
  try {
    if (isCdnAvailable.value) mirror_url = await GetCdnUrl();
    else mirror_url = selectedMirror.value!.url;
  } catch (e) {
    alert(e);
  }
  if (!mirror_url) {
    step.value = 3;
    return;
  }
  console.log(mirror_url);
  let total_downloaded_size = 0;
  const total_size = await invoke<number>('head_package', {
    mirrorUrl: mirror_url,
  });
  let stat: InstallStat = {
    speedLastSize: 0,
    lastTime: performance.now(),
    speed: 0,
  };
  progressInterval.value = setInterval(() => {
    const now = performance.now();
    const time_diff = now - stat.lastTime;
    if (time_diff > 100) {
      stat.speed = (total_downloaded_size - stat.speedLastSize) / time_diff;
      stat.speedLastSize = total_downloaded_size;
      stat.lastTime = now;
    }
    const speed = formatSize(stat.speed * 1000);
    const downloaded = formatSize(total_downloaded_size);
    const total = formatSize(total_size);
    current.value = `
      <span class="d-single-stat">${downloaded} / ${total} (${speed}/s)</span>
    `;
    percent.value = (total_downloaded_size / total_size) * 40;
  }, 30);

  let id = uuid();
  let unlisten = await listen<[number, number]>(id, ({ payload }) => {
    total_downloaded_size = payload[0];
  });
  await invoke('download_package', { mirrorUrl: mirror_url, id: id });
  unlisten();
  clearInterval(progressInterval.value);
  percent.value = 40;
  subStep.value = 1;
  current.value = t('正在检查 MSVC 运行库……');
  let is_vcrt_installed = await invoke<boolean>('check_vcrt');
  if (!is_vcrt_installed) {
    current.value = t('正在安装 MSVC 运行库……');
    id = uuid();
    unlisten = await listen<[number, number]>(id, ({ payload }) => {
      const currentSize = formatSize(payload[0]);
      const targetSize = payload[1] ? formatSize(payload[1]) : '';
      if (payload[0] >= payload[1] - 1) {
        current.value = t('安装 MSVC 运行库……');
      } else {
        current.value = t('下载 MSVC 运行库 ……x', [
          `<br>${currentSize}${targetSize ? ` / ${targetSize}` : ''}`,
        ]);
      }
    });
    await invoke('install_vcrt', { id: id });
    unlisten();
  }
  percent.value = 50;
  current.value = t('正在检查 GlobalSign Code Signing Root R45 证书……');
  try {
    await invoke('check_globalsign_r45');
  } catch (e) {
    alert(e);
    // todo: 后续处理
  }
  percent.value = 60;
  subStep.value = 2;
  current.value = t('正在部署包……');
  id = uuid();
  unlisten = await listen<number>(id, ({ payload }) => {
    current.value = `
      <span class="d-single-stat">${t('部署进度')}: ${payload} %</span>
    `;
    percent.value = 60 + payload * 0.39;
  });
  try {
    await invoke('install_package', { sha256: sha256.value, id: id });
  } catch (e) {
    alert(e);
  }
  unlisten();

  percent.value = 99;
  current.value = t('很快就好……');

  if (createLnk.value) {
    try {
      await invoke('create_desktop_lnk');
    } catch (e) {
      alert(e);
    }
  }
  await invoke('clear_temp_dir');

  current.value = t('安装完成');
  step.value = 5;
  percent.value = 100;
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
    testers.push(
      invoke<number>('speedtest_5mb', { url: mirror.url }).then(
        (s) => (mirror.speed = s),
      ),
    );
  }

  await Promise.all(testers);
  mirrors.value = mirrors.value.sort(
    (a, b) => (b.speed ?? -1) - (a.speed ?? -1),
  );
  selectedMirror.value = mirrors.value[0];
}

onMounted(async () => {
  const win = getCurrentWindow();
  await win.setTitle('Snap Hutao Deployment');
  await win.show();

  let config = await invoke<Config>('get_config');
  Object.assign(CONFIG, config);
  let patch_data = await fetchPatchData();
  mirrors.value = patch_data.mirrors;
  sha256.value = patch_data.sha256;
  isOversea.value = await fetchIsOversea();

  if (config.is_update && config.curr_version) {
    let local = Version.parse(config.curr_version);
    let remote = Version.parse(patch_data.version);
    if (remote.compare(local) <= 0) {
      step.value = 6;
      init.value = true;
      return;
    }

    version_info.value = `${local.toString()} -> ${remote.toString()}`;
    changelog.value = await invoke<string>('get_changelog', {'lang': getLang(), 'from': local.toString()});
  }

  testMirrorSpeed().catch((e) => alert(e));
  init.value = true;
});

function formatSize(size: number): string {
  if (size < 1024) {
    return `${size.toFixed(2)} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(2)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(2)} MB`;
}

class Version {
  major: number;
  minor: number;
  build: number;
  revision: number;

  constructor(
    major: number,
    minor: number,
    build: number | undefined,
    revision: number | undefined,
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
    const [major, minor, build, revision] = version.split('.').map(Number);
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
