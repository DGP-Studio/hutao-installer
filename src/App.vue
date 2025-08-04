<template>
  <div class="main">
    <div v-show="!init" class="init-loading">
      <span class="fui-Spinner__spinner">
        <span class="fui-Spinner__spinnerTail"></span>
      </span>
      <div class="init-self-updating" v-show="selfUpdating">{{ t('正在更新安装器……') }}</div>
      <div v-show="selfUpdateFailed" class="init-self-updating">
        {{ t('更新安装器失败，是否重试') }}
      </div>
      <div v-show="selfUpdateFailed" class="init-self-updating">
        {{ selfUpdateError }}
      </div>
      <div v-show="selfUpdateFailed" class="init-self-update-failed">
        <button class="btn btn-update-failed" @click="setSelfUpdateRetry(true)">
          <span>{{ t('重试') }}</span>
        </button>
        <button class="btn btn-update-failed" @click="setSelfUpdateRetry(false)">
          <span>{{ t('跳过') }}</span>
        </button>
      </div>
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
          <div class="sub-container">
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
              <span>{{ t('更新信息: x', [version_info]) }}</span>
              <vue-markdown :source="changelog" class="changelog" @click="handleMarkdownClick" />
            </div>
          </div>
          <div class="new-btn-container">
            <button :disabled="(!CONFIG.is_update && !acceptEula) || starting" class="btn new-btn" @click="start">
              <span v-if="starting" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else>{{ t('开始') }}</span>
            </button>
          </div>
        </div>
        <div class="login" v-if="step === 2">
          <div class="sub-container">
            <div class="desc">
              <span>
                {{ t('登录') }}
              /
              <a @click="goToRegister">{{ t('注册') }}</a>
              {{ t('以使用') }}
              <a @click="openAfdianPage">{{ t('胡桃云 CDN 服务') }}</a>
              {{ t('获取更好的下载体验') }}
              </span>
            </div>
            <input v-model="homaUsername" :placeholder="t('邮箱')" aria-autocomplete="none" autocomplete="off"
                   class="account-input"
                   type="email" />
            <input v-model="homaPassword" :placeholder="t('密码')" aria-autocomplete="none" autocomplete="off"
                   class="account-input textarea-password"
                   type="password" />
          </div>
          <div class="new-btn-container">
            <button class="btn new-btn" @click="loginSkip">
              {{ t('返回') }}
            </button>
            <button :disabled="!emailRegex.test(homaUsername) ||
              homaPassword.length === 0 ||
              logging_in
              " class="btn new-btn" @click="login">
              <span v-if="logging_in" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else>{{ t('登录') }}</span>
            </button>
          </div>
        </div>
        <div v-if="step === 7" class="register">
          <div class="sub-container">
            <div class="desc">
              <span>
               <a @click="gotoLogin"> {{ t('登录') }}</a>
              /
              {{ t('注册') }}
              {{ t('以使用') }}
              <a @click="openAfdianPage">{{ t('胡桃云 CDN 服务') }}</a>
              {{ t('获取更好的下载体验') }}
              </span>
            </div>
            <input v-model="homaUsername" :placeholder="t('邮箱')" autocomplete="off" class="account-input"
                   aria-autocomplete="none"
                   type="email" />
            <div class="verify-code-container">
              <input v-model="homaVerifyCode" :placeholder="t('验证码')" aria-autocomplete="none" autocomplete="off"
                     class="account-input verify-code-input"
                     type="text" />
              <button :disabled="requestingVerifyCode || verifyCodeCooldown || !emailRegex.test(homaUsername)"
                      class="btn btn-req-verify-code" @click="requestVerifyCode">
                <span v-if="requestingVerifyCode" class="fui-Spinner__spinner">
                  <span class="fui-Spinner__spinnerTail" />
                </span>
                <span v-else-if="verifyCodeCooldown">
                  {{ t('获取: x', [verifyCodeCountdown]) }}
                </span>
                <span v-else>{{ t('获取') }}</span>
              </button>
            </div>
            <input v-model="homaPassword" :placeholder="t('密码')" aria-autocomplete="none" autocomplete="off"
                   class="account-input textarea-password"
                   type="password" />
            <input v-model="homaRedeemCode" :placeholder="t('胡桃云兑换码')" autocomplete="off" class="account-input"
                   aria-autocomplete="none"
                   type="text" />
          </div>
          <div class="new-btn-container">
            <button class="btn new-btn" @click="loginSkip">
              {{ t('返回') }}
            </button>
            <button :disabled="!emailRegex.test(homaUsername) ||
            homaVerifyCode.length !== 8 ||
              homaPassword.length === 0 ||
              homaRedeemCode.length !== 18 ||
              registering
              " class="btn new-btn" @click="register">
              <span v-if="registering" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else>{{ t('注册') }}</span>
            </button>
          </div>
        </div>
        <div class="choose-mirror" v-if="step === 3">
          <div class="choose-mirror-desc">
            <div class="desc">
              {{ t('选择一个镜像源') }}
              <a @click="checkCdnPermission"> {{ t('已购买胡桃云 CDN？') }} </a>
            </div>
            <div class="listview">
              <div v-for="(item, index) in mirrors" :key="index" class="listview-item"
                   :class="{ selected: selectedMirror === item }" @click="onItemClick(item)">
                <div class="left-indicator" />
                <div class="mirror-item">
                  <span>{{ item.mirror_name }}</span>
                  <span v-if="item.mirror_type != 'browser'">
                    <button
                      v-if="item.speed != null"
                      :title="t('点击重新测速')"
                      class="speed-result-btn"
                      @click="(e) => onSpeedResultClick(item, e)"
                    >
                      <span class="speed-text">
                        {{
                          item.speed == -1
                            ? 'timeout'
                            : `${item.speed?.toFixed(2)} MB/s`
                        }}
                      </span>
                      <span class="refresh-icon">
                        <svg fill="none" height="12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"
                             stroke-width="2" viewBox="0 0 24 24" width="12">
                          <path d="M23 4v6h-6"></path>
                          <path d="M1 20v-6h6"></path>
                          <path d="m3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
                        </svg>
                      </span>
                    </button>
                    <span v-else>{{ t('测速中') }}</span>
                  </span>
                </div>
              </div>
            </div>
          </div>
          <div class="new-btn-container">
            <button v-if="selectedMirror?.mirror_type == 'browser'" :disabled="!selectedMirror || checking"
                    class="btn new-btn"
                    @click="openBrowserMirror">
              <span v-if="checking" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else>{{ t('跳转到浏览器') }}</span>
            </button>
            <button v-else :disabled="!selectedMirror || checking" class="btn new-btn" @click="install">
              <span v-if="checking" class="fui-Spinner__spinner">
                <span class="fui-Spinner__spinnerTail" />
              </span>
              <span v-else>{{ CONFIG.is_update ? t('更新') : t('安装') }}</span>

            </button>
          </div>
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
              <a v-if="suggestOffline && subStep == 0" @click="openOfflineDownloadPage"> {{ t('下载很慢？试试离线包')
                }} </a>
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
          <div class="new-btn-container">
            <button class="btn new-btn" @click="launch">
              {{ t('启动') }}
            </button>
          </div>
        </div>
        <div class="finish" v-if="step === 6">
          <div class="finish-text">
            <CircleSuccess />
            <span>{{ t('您已安装最新版本') }}</span>
          </div>
          <div class="new-btn-container">
            <button class="btn new-btn" @click="restart">
              {{ t('重新安装') }}
            </button>
            <button class="btn new-btn" @click="launch">
              {{ t('启动') }}
            </button>
          </div>
        </div>
      </div>
    </div>
    <div v-show="init" class="version">{{ CONFIG.version
      }}{{ CONFIG.embedded_version ? `/${CONFIG.embedded_version}` : '' }}
    </div>
  </div>
</template>

<style scoped>
.main {
  height: 100vh;
  max-height: 100vh;
}

.version {
  position: absolute;
  bottom: 10px;
  left: 10px;
  opacity: 0.8;
  font-size: 12px;
  pointer-events: none;
}

.init-loading {
  height: 100vh;
  display: flex;
  flex-direction: column;
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

.init-self-updating {
  margin-top: 16px;
  font-size: 14px;
}

.init-self-update-failed {
  margin-top: 16px;
  font-size: 14px;
  display: flex;
  justify-content: center;
  gap: 16px;
}

.content {
  display: flex;
  height: 100vh;
  max-height: 100vh;
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
  display: flex;
  gap: 8px;
  justify-content: space-between;

  a {
    cursor: pointer;
  }
}

.account-input {
  height: 32px;
  padding: 8px;
  background: var(--colorTextareaBackground);
  color: var(--colorTextareaText);
  border-radius: 4px;
  box-sizing: border-box;
  font-size: 12px;
  resize: none;
  opacity: 0.8;
  margin-left: 10px;
  margin-bottom: 4px;
  font-family: Consolas,
  'Courier New',
  Microsoft Yahei,
  serif;
  border: unset;
  outline: none;
}

.verify-code-input {
  width: 100%;
}

.textarea-password {
  -webkit-text-security: disc;
}

.image {
  min-width: 300px;
  width: 300px;
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
  padding: 24px;
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

.new-btn-container {
  display: flex;
  height: 40px;
  flex-shrink: 0;
  gap: 10px;
  margin-left: 10px;
  justify-content: space-between;

  .fui-Spinner__spinner {
    width: 16px;
    height: 16px;
    display: block;
  }
}

.new-btn {
  height: 40px;
  width: 100%;
}

.verify-code-container {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  gap: 8px;

  .fui-Spinner__spinner {
    width: 16px;
    height: 16px;
    display: block;
  }
}

.btn-req-verify-code {
  height: 32px;
  margin-bottom: 4px;
  vertical-align: center;
}

.btn-update-failed {
  height: 40px;
  width: 100px;
}

.sub-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.actions,
.login,
.register,
.choose-mirror,
.finish {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
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
  height: 100%;
  line-height: 1.4;
  font-size: 13px;
  gap: 8px;
  flex-direction: column;
}

.changelog {
  flex: 1 1 0;
  overflow-y: auto;
  margin-left: -20px;

  ::v-deep(h3) {
    margin-top: 0;
    margin-left: 20px;
  }
}

.changelog::-webkit-scrollbar {
  width: 4px;
  height: 4px;
  border-radius: 4px;
  background: transparent;
}

.finish-text {
  height: 100%;
  text-align: center;
  opacity: 0.9;
  padding: 0 10px 38px;
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
  font-size: 14px;
  display: flex;
  height: 100%;
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

  a {
    cursor: pointer;
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
  font-family: Consolas,
  'Courier New',
  Microsoft Yahei,
  serif;
}

.listview {
  flex: 1 1 0;
  overflow-y: auto;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.listview::-webkit-scrollbar {
  width: 4px;
  height: 4px;
  border-radius: 4px;
  background: transparent;
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

.speed-result-btn {
  position: relative;
  display: inline;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  padding: 0;
  margin: 0;
  cursor: pointer;
  font-size: 14px;
  font-family: inherit;
  font-weight: inherit;
  line-height: inherit;
  color: inherit;
  border-radius: 4px;
  transition: all 0.2s ease;
  overflow: hidden;
  vertical-align: baseline;
}

.speed-result-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  margin: -2px;
  padding: 2px;
}

.speed-result-btn .speed-text {
  transition: opacity 0.2s ease;
  z-index: 1;
}

.speed-result-btn .refresh-icon {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  opacity: 0;
  font-size: 12px;
  transition: opacity 0.2s ease;
  background: transparent;
  z-index: 2;
}

.speed-result-btn:hover .speed-text {
  opacity: 0.3;
}

.speed-result-btn:hover .refresh-icon {
  opacity: 1;
  transform: translate(-50%, -50%);
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
import { onMounted, reactive, Ref, ref } from 'vue';
import { getCurrentWindow, invoke, listen } from './tauri';
import Checkbox from './components/Checkbox.vue';
import CircleSuccess from './components/CircleSuccess.vue';
import { v4 as uuid } from 'uuid';
import {
  fetchIsOversea,
  fetchPatchData,
  GetCdnUrl,
  GetUserInfo,
  IsCdnAvailable,
  IsLoggedIn,
  LoadToken,
  LoginHomaPassport,
  Logout,
  RegisterHomaPassportAndUseRedeemCode,
  RequestHomaPassportVerifyCode,
} from './api';
import { getLang } from './i18n';

const { t } = useI18n();

// Process init
const init = ref(false);
const selfUpdating = ref(false);
const selfUpdateFailed = ref(false);
const selfUpdateRetry = ref<boolean | null>(null);
const selfUpdateError = ref<string | null>(null);

const subStepList: ReadonlyArray<string> = [
  t('准备安装包'),
  t('准备运行环境'),
  t('部署文件'),
];

/**
 * 1: EULA
 * 2: Login
 * 3: Choose Mirror
 * 4: Install
 * 5: Finish
 * 6: Already Installed
 * 7: Register
 */
const step = ref<number>(1);
const subStep = ref<number>(0);

// Step 1
const acceptEula = ref<boolean>(true);
const createLnk = ref<boolean>(true);
const starting = ref<boolean>(false);
const version_info = ref<string>('');
const changelog = ref<string>('');
let isCdnAvailable = false;
let isOversea = false;
let remote_version = '';

// Step 2
const homaUsername = ref<string>('');
const homaPassword = ref<string>('');
const logging_in = ref<boolean>(false);

// Step 3
const mirrors = ref<GenericPatchPackageMirror[]>([]);
const selectedMirror = ref<GenericPatchPackageMirror | null>(null);
const checking = ref<boolean>(false);

// Step 4
const current = ref<string>('');
const percent = ref<number>(0);

const suggestOffline = ref<boolean>(false);
let sha256 = '';

// Step 7
const homaVerifyCode = ref<string>('');
const homaRedeemCode = ref<string>('');

const requestingVerifyCode = ref<boolean>(false);
const verifyCodeCooldown = ref<boolean>(false);
const verifyCodeCountdown = ref<number>(0);
const registering = ref<boolean>(false);

// Intervals
let headingPackageInterval = 0;
let progressInterval = 0;
let verifyCodeInterval = 0;

let embedded_is_latest = false;
const CONFIG: Config = reactive({
  version: '0.0.0',
  is_update: false,
  need_migration: false,
  skip_self_update: false,
  is_offline_mode: false,
  embedded_version: null,
  curr_version: null,
  token: null,
});

const emailRegex = /^[\w-]+(\.[\w-]+)*@[\w-]+(\.[\w-]+)+$/;

async function openTos(): Promise<void> {
  await invoke('open_browser', { url: 'https://hut.ao/statements/tos.html' });
}

async function start(): Promise<void> {
  starting.value = true;

  if (CONFIG.is_offline_mode) {
    if (embedded_is_latest) {
      await install();
      starting.value = false;
      return;
    }

    if (await invoke<boolean>('confirm_dialog', {
      'title': t('提示'),
      'message': t('此离线安装包不是最新版本，是否继续安装？取消以在线安装最新版本'),
    })) {
      embedded_is_latest = true;
      await install();
      starting.value = false;
      return;
    }
  }

  if (isOversea) {
    selectedMirror.value = mirrors.value[0];
    await install();
    starting.value = false;
    return;
  }

  if (isCdnAvailable || await IsCdnAvailable()) {
    await install();
    starting.value = false;
    return;
  }

  if (CONFIG.token) {
    await LoadToken(CONFIG.token);
    if (await IsCdnAvailable()) {
      isCdnAvailable = true;
      await install();
      starting.value = false;
      return;
    }
  }

  step.value = 3;
  starting.value = false;
}

async function checkCdnPermission(): Promise<void> {
  checking.value = true;
  if (await IsLoggedIn()) {
    if (await IsCdnAvailable()) {
      isCdnAvailable = true;
      await install();
    } else {
      const userInfo = await GetUserInfo();
      const action = await invoke<[boolean, boolean]>('three_btn_custom_dialog', {
        title: t('无 CDN 权限'),
        message: t('当前登录账号x未检测到有效 CDN 权限', [userInfo.UserName]),
        yes: t('前往赞助页面'),
        no: t('退出当前账号'),
        cancel: t('取消'),
      });

      if (action[0]) {
        await openAfdianPage();
      } else if (action[1]) {
        isCdnAvailable = false;
        await Logout();
        step.value = 2;
      }
    }
  } else {
    step.value = 2;
  }
  checking.value = false;
}

async function gotoLogin(): Promise<void> {
  step.value = 2;
}

async function login(): Promise<void> {
  logging_in.value = true;
  if (!(await LoginHomaPassport(homaUsername.value, homaPassword.value))) {
    logging_in.value = false;
    homaPassword.value = '';
    return;
  }
  if (await IsCdnAvailable()) {
    isCdnAvailable = true;
    await install();
  } else {
    const open_afdian_page_action = await invoke<boolean>('two_btn_custom_dialog', {
      title: t('无 CDN 权限'),
      message: t('未检测到有效 CDN 权限，请选择一个镜像源进行下载安装包，或前往赞助页面获取 CDN 权限'),
      ok: t('前往赞助页面'),
      cancel: t('选择镜像源'),
    });
    if (open_afdian_page_action) {
      await openAfdianPage();
      logging_in.value = false;
      return;
    } else {
      step.value = 3;
    }
  }
  logging_in.value = false;
  homaUsername.value = '';
  homaPassword.value = '';
}

async function register(): Promise<void> {
  registering.value = true;
  if (!(await RegisterHomaPassportAndUseRedeemCode(homaUsername.value, homaPassword.value, homaVerifyCode.value, homaRedeemCode.value))) {
    registering.value = false;
    return;
  }
  if (await IsCdnAvailable()) {
    isCdnAvailable = true;
    await install();
  } else {
    const open_afdian_page_action = await invoke<boolean>('two_btn_custom_dialog', {
      title: t('无 CDN 权限'),
      message: t('当前兑换码为抽卡记录兑换码，未检测到有效 CDN 权限，请选择一个镜像源进行下载安装包，或前往赞助页面获取 CDN 权限'),
      ok: t('前往赞助页面'),
      cancel: t('选择镜像源'),
    });

    if (open_afdian_page_action) {
      await openAfdianPage();
      registering.value = false;
      homaVerifyCode.value = '';
      homaRedeemCode.value = '';
      await gotoLogin();
      return;
    } else {
      step.value = 3;
    }
  }
  registering.value = false;
  homaUsername.value = '';
  homaPassword.value = '';
  homaVerifyCode.value = '';
  homaRedeemCode.value = '';
}

async function requestVerifyCode(): Promise<void> {
  if (homaUsername.value.length === 0) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('请输入邮箱'),
    });
    return;
  }

  if (!emailRegex.test(homaUsername.value)) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('请输入正确的邮箱地址'),
    });
    return;
  }

  requestingVerifyCode.value = true;
  if (!(await RequestHomaPassportVerifyCode(homaUsername.value))) {
    requestingVerifyCode.value = false;
    return;
  }
  verifyCodeCooldown.value = true;
  requestingVerifyCode.value = false;
  verifyCodeCountdown.value = 60;
  verifyCodeInterval = setInterval(() => {
    if (verifyCodeCountdown.value > 0) {
      verifyCodeCountdown.value -= 1;
    } else {
      clearInterval(verifyCodeInterval);
      verifyCodeCooldown.value = false;
    }
  }, 1000);
}

async function loginSkip(): Promise<void> {
  homaUsername.value = '';
  homaPassword.value = '';
  homaVerifyCode.value = '';
  homaRedeemCode.value = '';
  step.value = 3;
}

async function goToRegister(): Promise<void> {
  verifyCodeCooldown.value = false;
  verifyCodeCountdown.value = 0;
  clearInterval(verifyCodeInterval);
  step.value = 7;
}

async function openAfdianPage(): Promise<void> {
  await invoke('open_browser', { url: 'https://afdian.com/item/274f5a7cbe9911efab6552540025c377' });
}

async function openOfflineDownloadPage(): Promise<void> {
  await invoke('open_browser', { url: 'https://pan.quark.cn/s/d73ceb415ad9#/list/share/1e5419a0b7554f98a9b218cf4d735f4b-%E8%83%A1%E6%A1%83/e4be2335e57d4328b8caeb54aaff08e6-%E7%A6%BB%E7%BA%BF%E5%8C%85' });
}

async function openBrowserMirror(): Promise<void> {
  await invoke('open_browser', { url: selectedMirror.value?.url });
}

async function install(): Promise<void> {
  step.value = 4;
  percent.value = 0;
  if (embedded_is_latest) {
    current.value = t('准备中……');
    try {
      await invoke('extract_package');
    } catch (e) {
      await invoke('error_dialog', {
        title: t('错误'),
        message: t('提取安装包失败，请重试') + '\n\n' + e,
      });
      step.value = 1;
      return;
    }
  } else {
    current.value = t('准备下载……');
    const package_exists_and_valid = await invoke<boolean>('check_temp_package_valid', { 'sha256': sha256 });
    if (!package_exists_and_valid) {
      let mirror_url;
      try {
        mirror_url = isCdnAvailable ? await GetCdnUrl(`Snap.Hutao.${remote_version}.msix`) : selectedMirror.value!.url;
      } catch (e) {
        await invoke('error_dialog', {
          title: t('错误'),
          message: t('未获取到可用的镜像源，请重试'),
        });
        step.value = 1;
        return;
      }
      let total_downloaded_size = 0;
      headingPackageInterval = setInterval(() => {
        if (!isOversea) {
          suggestOffline.value = true;
        }

        clearInterval(headingPackageInterval);
      }, 5000);
      const total_size = await invoke<number>('head_package', {
        mirrorUrl: mirror_url,
      });
      let stat: InstallStat = {
        speedLastSize: 0,
        lastTime: performance.now(),
        speed: 0,
        lowSpeedCount: 0,
      };
      progressInterval = setInterval(() => {
        clearInterval(headingPackageInterval);
        const now = performance.now();
        const time_diff = now - stat.lastTime;
        if (time_diff > 500) {
          stat.speed = (total_downloaded_size - stat.speedLastSize) / time_diff;
          stat.speedLastSize = total_downloaded_size;
          stat.lastTime = now;

          if ((stat.speed * 1000) < (800 * 1000)) {
            stat.lowSpeedCount += 1;
          }

          if (!isOversea && stat.lowSpeedCount > 10) {
            suggestOffline.value = true;
          }
        }
        const speed = formatSize(stat.speed * 1000);
        const downloaded = formatSize(total_downloaded_size);
        const total = formatSize(total_size);
        current.value = `<span class="d-single-stat">${downloaded} / ${total} (${speed}/s)</span>`;
        percent.value = (total_downloaded_size / total_size) * 40;
      }, 30);

      let id = uuid();
      let unlisten = await listen<[number, number]>(id, ({ payload }) => {
        total_downloaded_size = payload[0];
      });
      try {
        await invoke('download_package', { mirrorUrl: mirror_url, id: id });
      } catch (e) {
        await invoke('error_dialog', {
          title: t('错误'),
          message: t('下载安装包失败，请重试') + '\n\n' + e,
        });
        step.value = 1;
        return;
      } finally {
        unlisten();
        clearInterval(progressInterval);
      }
    }
  }
  percent.value = 40;

  subStep.value = 1;
  current.value = t('正在检查 MSVC 运行库……');
  let is_vcrt_installed = await invoke<boolean>('check_vcrt');
  if (!is_vcrt_installed) {
    current.value = t('正在安装 MSVC 运行库……');
    let id = uuid();
    let unlisten = await listen<[number, number]>(id, ({ payload }) => {
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
    try {
      await invoke('install_vcrt', { id: id });
    } catch (e) {
      await invoke('error_dialog', {
        title: t('错误'),
        message: t('安装 MSVC 运行库失败，请重试') + '\n\n' + e,
      });
      step.value = 1;
      return;
    } finally {
      unlisten();
    }
  }
  percent.value = 45;

  current.value = t('正在检查 GlobalSign Code Signing Root R45 证书……');
  try {
    await invoke('check_globalsign_r45');
  } catch (e) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('检查 GlobalSign Code Signing Root R45 证书失败，请重试') + '\n\n' + e,
    });
    step.value = 1;
    return;
  }
  percent.value = 50;

  current.value = t('正在检查 Segoe Fluent Icons 字体……');
  let is_segoe_fluent_icons_font_installed = await invoke<boolean>('check_segoe_fluent_icons_font');
  if (!is_segoe_fluent_icons_font_installed) {
    current.value = t('正在安装 Segoe Fluent Icons 字体……');
    try {
      await invoke('install_segoe_fluent_icons_font');
    } catch (e) {
      await invoke('error_dialog', {
        title: t('错误'),
        message: t('安装 Segoe Fluent Icons 字体失败，请重试') + '\n\n' + e,
      });
      step.value = 1;
      return;
    }
  }
  percent.value = 55;

  current.value = t('正在检查 Win32 长路径支持……');
  try {
    await invoke('check_win32_long_path_support');
  } catch (e) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('检查 Win32 长路径支持失败，请重试') + '\n\n' + e,
    });
    step.value = 1;
    return;
  }
  percent.value = 60;


  subStep.value = 2;
  current.value = t('正在部署包……');
  const hutao_running_state = await invoke<[boolean, number?]>('is_hutao_running');
  if (hutao_running_state[0]) {
    if (await invoke<boolean>('confirm_dialog', {
      'title': t('提示'),
      'message': t('检测到 Snap Hutao 正在运行，是否结束进程继续部署？'),
    })) {
      try {
        await invoke('kill_process', { 'pid': hutao_running_state[1] });
      } catch (e) {
        await invoke('message_dialog', {
          'title': t('提示'),
          'message': t('结束进程失败，请手动结束进程后再尝试部署' + '\n\n' + e),
        });
        step.value = 1;
        subStep.value = 0;
        return;
      }
    } else {
      await invoke('message_dialog', { 'title': t('提示'), 'message': t('请手动结束进程后再尝试部署') });
      step.value = 1;
      subStep.value = 0;
      return;
    }
  }

  if (CONFIG.need_migration) {
    if (await invoke<boolean>('confirm_dialog', {
      'title': t('提示'),
      'message': t('检测到不兼容的旧版本，安装程序将先卸载旧版本，数据不受影响，部分设置可能会丢失，是否继续？'),
    })) {
      try {
        current.value = t('正在卸载不兼容的旧版本……');
        await invoke('remove_outdated_package');
        current.value = t('正在部署包……');
      } catch (e) {
        await invoke('error_dialog', {
          title: t('错误'),
          message: t('旧版本卸载失败，请重试') + '\n\n' + e,
        });
        step.value = 1;
        subStep.value = 0;
        return;
      }
    } else {
      await invoke('message_dialog', { 'title': t('提示'), 'message': t('请先手动卸载旧版本后再重新部署') });
      step.value = 1;
      subStep.value = 0;
      return;
    }
  }

  let id = uuid();
  let unlisten = await listen<number>(id, ({ payload }) => {
    current.value = `
      <span class="d-single-stat">${t('部署进度')}: ${payload} %</span>
    `;
    percent.value = 60 + payload * 0.39;
  });
  try {
    if (!await invoke<boolean>('install_package', { sha256: sha256, id: id, offlineMode: embedded_is_latest })) {
      step.value = 1;
      subStep.value = 0;
      return;
    }
  } catch (e) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('部署包失败，请重试') + '\n\n' + e,
    });
    step.value = 1;
    subStep.value = 0;
    return;
  } finally {
    unlisten();
  }

  percent.value = 99;
  current.value = t('很快就好……');

  if (createLnk.value && !CONFIG.is_update) {
    try {
      await invoke('create_desktop_lnk');
    } catch (e) {
      await invoke('error_dialog', {
        title: t('错误'),
        message: t('创建桌面快捷方式失败') + '\n\n' + e,
      });
    }
  }

  current.value = t('安装完成');
  step.value = 5;
  percent.value = 100;
}

async function restart(): Promise<void> {
  let config = {
    is_update: false,
    curr_version: null,
    token: CONFIG.token,
  };
  Object.assign(CONFIG, config);
  testMirrorSpeed().catch((e) => alert(e));
  step.value = 1;
}

async function launch(): Promise<void> {
  await invoke('launch_and_exit');
}

function onItemClick(item: GenericPatchPackageMirror): void {
  selectedMirror.value = item;
}

async function onSpeedResultClick(item: GenericPatchPackageMirror, event: Event): Promise<void> {
  event?.stopPropagation();

  item.speed = null;
  await invoke<number>('speedtest_5mb', { url: item.url }).then(
    (s) => (item.speed = s),
  );

  mirrors.value = mirrors.value.sort(
    (a, b) => (b.speed ?? -1) - (a.speed ?? -1),
  );
}

async function testMirrorSpeed(): Promise<void> {
  const testers = [];
  for (const mirror of mirrors.value) {
    if (mirror.mirror_type != 'direct') {
      continue;
    }
    mirror.speed = null;
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

async function handleMarkdownClick(e: MouseEvent): Promise<void> {
  const target = (e.target as HTMLElement).closest('a');
  if (!target) {
    return;
  }

  e.preventDefault();
  const href = target.getAttribute('href');
  if (href) {
    await invoke('open_browser', { url: href });
  }
}

async function setSelfUpdateRetry(value: boolean): Promise<void> {
  selfUpdateRetry.value = value;
}

async function waitForRefValid(ref: Ref) {
  return new Promise((resolve) => {
    const interval = setInterval(() => {
      if (ref.value !== null) {
        clearInterval(interval);
        resolve(ref.value);
      }
    }, 100);
  });
}

onMounted(async () => {
  const win = getCurrentWindow();
  await win.setTitle('Snap Hutao Deployment');
  await win.show();

  let config = await invoke<Config>('get_config');
  Object.assign(CONFIG, config);
  let patch_data: GenericPatchData;
  try {
    patch_data = await fetchPatchData();
    isOversea = await fetchIsOversea();
  } catch (e) {
    await invoke('error_dialog', {
      title: t('错误'),
      message: t('无法连接到胡桃 API，请检查网络后重启安装器') + '\n\n' + e,
    });
    await invoke('exit');
    return;
  }
  mirrors.value = patch_data.mirrors;
  sha256 = patch_data.sha256;
  remote_version = Version.parse(patch_data.version).toString();

  if (!config.skip_self_update) {
    if (await invoke<boolean>('need_self_update')) {
      while (true) {
        selfUpdating.value = true;
        selfUpdateFailed.value = false;
        selfUpdateRetry.value = null;
        try {
          await invoke('self_update');
        } catch (e) {
          selfUpdating.value = false;
          selfUpdateFailed.value = true;
          selfUpdateError.value = e as string;
          await waitForRefValid(selfUpdateRetry);
          // noinspection PointlessBooleanExpressionJS
          if (selfUpdateRetry.value === false) {
            break;
          }
        }
      }
    }
  }

  if (config.is_offline_mode) {
    if (!config.embedded_version) throw new Error('Never happen');
    let embed_ver = Version.parse(config.embedded_version);
    let remote = Version.parse(patch_data.version);
    if (remote.compare(embed_ver) <= 0) {
      embedded_is_latest = true;
    }
  }

  if (!isOversea) {
    mirrors.value.push({
      url: 'https://pan.quark.cn/s/d73ceb415ad9#/list/share/1e5419a0b7554f98a9b218cf4d735f4b-%E8%83%A1%E6%A1%83/e4be2335e57d4328b8caeb54aaff08e6-%E7%A6%BB%E7%BA%BF%E5%8C%85',
      mirror_name: t('夸克网盘'),
      mirror_type: 'browser',
      speed: 0,
    });
  }

  if (config.is_update && config.curr_version) {
    let local = Version.parse(config.curr_version);
    let remote = Version.parse(patch_data.version);
    if (remote.compare(local) <= 0) {
      step.value = 6;
      init.value = true;
      return;
    }

    version_info.value = `${local.toString()} -> ${remote.toString()}`;
    changelog.value = await invoke<string>('get_changelog', { 'lang': getLang(), 'from': local.toString() });
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
    return `${this.major}.${this.minor}.${this.build}`;
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
