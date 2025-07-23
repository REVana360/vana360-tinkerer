
interface ImportMetaEnv {
    [key: string]: any
    BASE_URL: string
    MODE: string
    DEV: boolean
    PROD: boolean
    SSR: boolean
}

interface ImportMeta {
    readonly env: ImportMetaEnv
}
