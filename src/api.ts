import { invoke } from './tauri.ts';

// @ts-expect-error crypto will be there
import crypto from 'crypto';
import { formatLocalizedString, getLocalizedString } from './i18n';

const PUBLIC_KEY = `-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA5W2SEyZSlP2zBI1Sn8Gd
TwbZoXlUGNKyoVrY8SVYu9GMefdGZCrUQNkCG/Np8pWPmSSEFGd5oeug/oIMtCZQ
NOn0drlR+pul/XZ1KQhKmj/arWjN1XNok2qXF7uxhqD0JyNT/Fxy6QvzqIpBsM9S
7ajm8/BOGlPG1SInDPaqTdTRTT30AuN+IhWEEFwT3Ctv1SmDupHs2Oan5qM7Y3uw
b6K1rbnk5YokiV2FzHajGUymmSKXqtG1USZzwPqImpYb4Z0M/StPFWdsKqexBqMM
mkXckI5O98GdlszEmQ0Ejv5Fx9fR2rXRwM76S4iZTfabYpiMbb4bM42mHMauupj6
9QIDAQAB
-----END PUBLIC KEY-----`;

let cachedData: GenericPatchData | null = null;
let cachedToken: string | null = null;

export async function fetchIsOversea(): Promise<boolean> {
  return await invoke<boolean>('generic_is_oversea');
}

export async function fetchPatchData(): Promise<GenericPatchData> {
  if (cachedData) {
    return cachedData;
  }

  const data = await invoke<GenericPatchData>('generic_get_patch');
  if (!data) {
    throw new Error('Failed to fetch patch data');
  }
  if (!data.mirrors) {
    throw new Error('Invalid patch data');
  }
  cachedData = data;
  return data;
}

export async function RequestHomaPassportVerifyCode(username: string): Promise<boolean> {
  const res = await invoke<HomaResp>('homa_request_verify_code', {
    username: encrypt(username),
  });
  if (res.retcode === 0) {
    return true;
  } else if (res.retcode === 544001) {
    await invoke('error_dialog', {
      title: getLocalizedString('请求验证码失败'),
      message: formatLocalizedString('邮箱 x 已被注册', [username]),
    });
    return false;
  } else {
    await invoke('error_dialog', {
      title: getLocalizedString('请求验证码失败'),
      message: res.message,
    });
    return false;
  }
}

export async function RegisterHomaPassportAndUseRedeemCode(
  username: string,
  password: string,
  verifyCode: string,
  redeemCode: string,
): Promise<boolean> {
  const req: HomaPassportRegisterReq = {
    UserName: encrypt(username),
    Password: encrypt(password),
    VerifyCode: encrypt(verifyCode),
  };
  const res = await invoke<HomaPassportOperationResp>('homa_register', {
    registerReq: req,
  });
  if (res.retcode !== 0) {
    await invoke('error_dialog', {
      title: getLocalizedString('注册失败'),
      message: res.message,
    });
    return false;
  }

  cachedToken = res.data ?? null;

  const redeemRes = await invoke<HomaPassportOperationResp>('homa_use_redeem_code', {
    token: cachedToken,
    code: redeemCode,
  });
  if (redeemRes.retcode !== 0) {
    await invoke('error_dialog', {
      title: getLocalizedString('兑换码使用失败'),
      message: formatLocalizedString('注册成功，但是x', [redeemRes.message]),
    });
    return false;
  }

  return true;
}

export async function LoginHomaPassport(
  username: string,
  password: string,
): Promise<boolean> {
  const req: HomaPassportLoginReq = {
    UserName: encrypt(username),
    Password: encrypt(password),
  };
  const res = await invoke<HomaPassportOperationResp>('homa_login', {
    loginReq: req,
  });
  if (res.retcode === 0) {
    cachedToken = res.data ?? null;
    return true;
  }

  await invoke('error_dialog', {
    title: getLocalizedString('登录失败'),
    message: res.message,
  });
  return false;
}

export async function IsLoggedIn(): Promise<boolean> {
  return !!cachedToken;
}

export async function Logout(): Promise<void> {
  cachedToken = null;
}

export async function LoadToken(token: string): Promise<void> {
  cachedToken = token;
}

export async function GetUserInfo(): Promise<HomaPassportUserInfo> {
  return await invoke<HomaPassportUserInfo>('homa_fetch_userinfo', {
    token: cachedToken,
  });
}

export async function IsCdnAvailable(): Promise<boolean> {
  if (!cachedToken) {
    return false;
  }

  const userinfo = await invoke<HomaPassportUserInfo>('homa_fetch_userinfo', {
    token: cachedToken,
  });

  return (
    userinfo.IsLicensedDeveloper ||
    userinfo.IsMaintainer ||
    new Date(userinfo.CdnExpireAt) > new Date()
  );
}

export async function GetCdnUrl(filename: string): Promise<string> {
  if (!cachedToken) {
    throw new Error('Not logged in');
  }

  return await invoke<string>('homa_fetch_cdn', {
    token: cachedToken,
    filename: filename,
  });
}

function encrypt(data: string): string {
  // @ts-expect-error And Buffer will be there
  let encData = Buffer.alloc(1);
  while (encData.length !== 256) {
    encData = crypto.publicEncrypt(
      {
        key: PUBLIC_KEY,
        padding: crypto.constants.RSA_PKCS1_OAEP_PADDING,
      },
      // @ts-expect-error And Buffer will be there
      Buffer.from(data),
    );
  }

  return encData.toString('base64');
}
