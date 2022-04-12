export const idlFactory = ({ IDL }) => {
  return IDL.Service({
    'addContentModerator' : IDL.Func([IDL.Principal], [], []),
    'getAllIndexes' : IDL.Func([], [IDL.Vec(IDL.Principal)], ['query']),
    'getGlobalIndex' : IDL.Func([], [IDL.Vec(IDL.Vec(IDL.Text))], ['query']),
    'getIndexByTag' : IDL.Func([IDL.Text], [IDL.Vec(IDL.Principal)], ['query']),
    'getMetrics' : IDL.Func([], [IDL.Text], ['query']),
    'getUploadOrder' : IDL.Func([], [IDL.Vec(IDL.Principal)], ['query']),
  });
};
export const init = ({ IDL }) => { return []; };
