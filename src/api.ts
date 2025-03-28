import { invoke } from './tauri.ts';

// @ts-expect-error crypto will be there
import crypto from 'crypto';

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

export async function LoginHomaPassport(
  username: string,
  password: string,
): Promise<boolean> {
  const req: HomaPassportLoginReq = {
    UserName: encrypt(username),
    Password: encrypt(password),
  };
  const res = await invoke<HomaPassportLoginResp>('homa_login', {
    loginReq: req,
  });
  if (res.retcode === 0) {
    cachedToken = res.data ?? null;
    return true;
  }

  await invoke('error_dialog', {
    title: '登录失败',
    message: res.message,
  });
  return false;
}

export async function LoadToken(token: string): Promise<void> {
  cachedToken = token;
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

export async function GetCdnUrl(): Promise<string> {
  if (!cachedToken) {
    throw new Error('Not logged in');
  }

  if (!cachedData) {
    throw new Error('Patch data not fetched');
  }

  const filename = cachedData.urls[0].split('/').pop();
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
