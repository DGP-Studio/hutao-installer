type HomaPassportLoginResp = {
    retcode: number,
    message: string,
    data?: string,
}

type HomaPassportLoginReq = {
    UserName: string,
    Password: string,
}

type HomaPassportUserInfo = {
    NormalizedUserName?: string,
    UserName?: string,
    IsLicensedDeveloper: boolean,
    IsMaintainer: boolean,
    GachaLogExpireAt: string,
    CdnExpireAt: string,
}

type GenericPatchData = {
    version: string,
    validation: string,
    cache_time: string,
    mirrors: GenericPatchPackageMirror[],
    urls: string[],
    sha256: string,
}

type GenericPatchPackageMirror = {
    url: string,
    mirror_name: string,
    mirror_type: string,
}

interface Config {
    is_update: boolean;
    curr_version: string | null;
  token: string | null;
}
