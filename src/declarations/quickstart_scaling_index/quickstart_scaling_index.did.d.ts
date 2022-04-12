import type { Principal } from '@dfinity/principal';
export interface _SERVICE {
  'addContentModerator' : (arg_0: Principal) => Promise<undefined>,
  'getAllIndexes' : () => Promise<Array<Principal>>,
  'getGlobalIndex' : () => Promise<Array<Array<string>>>,
  'getIndexByTag' : (arg_0: string) => Promise<Array<Principal>>,
  'getMetrics' : () => Promise<string>,
  'getUploadOrder' : () => Promise<Array<Principal>>,
}
