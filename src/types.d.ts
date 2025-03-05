type HomaPassportLoginResp = {
  retcode: number;
  message: string;
  data?: string;
};

type HomaPassportLoginReq = { UserName: string; Password: string };

type HomaPassportUserInfo = {
  normalized_username?: string;
  username?: string;
  is_licensed_developer: boolean;
  is_maintainer: boolean;
  gacha_log_expire_at: string;
  cdn_expire_at: string;
};

type GenericPatchData = {
  version: string;
  validation: string;
  cache_time: string;
  mirrors: GenericPatchPackageMirror[];
  urls: string[];
  sha256: string;
};

type GenericPatchPackageMirror = {
  url: string;
  mirror_name: string;
  mirror_type: string;
  speed: number | null;
};

interface Config {
  is_update: boolean;
  curr_version: string | null;
  token: string | null;
}

type InstallStat = {
  speedLastSize: number;
  lastTime: DOMHighResTimeStamp;
  speed: number;
};
