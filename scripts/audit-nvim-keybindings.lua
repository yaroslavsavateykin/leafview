-- Read-only runtime inventory; run with the user's actual init.lua.
local root = assert(vim.env.LEAFVIEW_AUDIT_DIR)
vim.fn.mkdir(root, 'p')
local modes = {'n','i','v','x','s','o','c','t','l'}
local snapshots = {}
local function clean(map)
  local out = {}
  for k,v in pairs(map) do
    if type(v) ~= 'function' and type(v) ~= 'userdata' then out[k]=v end
  end
  if map.callback then
    local info=debug.getinfo(map.callback,'S') or {}
    out.callback_source=info.source
    out.callback_line=info.linedefined
  end
  if map.sid and map.sid > 0 then
    local info=vim.fn.getscriptinfo({sid=map.sid})
    if info[1] then out.script=info[1].name end
  end
  return out
end
local function snapshot(name)
  local sample={name=name,filetype=vim.bo.filetype,global={},buffer={},clients={}}
  for _,mode in ipairs(modes) do
    sample.global[mode]={};sample.buffer[mode]={}
    for _,map in ipairs(vim.api.nvim_get_keymap(mode)) do table.insert(sample.global[mode],clean(map)) end
    for _,map in ipairs(vim.api.nvim_buf_get_keymap(0,mode)) do table.insert(sample.buffer[mode],clean(map)) end
  end
  for _,client in ipairs(vim.lsp.get_clients({bufnr=0})) do
    table.insert(sample.clients,{name=client.name,initialized=client.initialized,capabilities=client.server_capabilities})
  end
  snapshots[#snapshots+1]=sample
end
snapshot('startup')
-- Trigger normal lazy lifecycle, not indiscriminately load disabled or optional plugins.
vim.api.nvim_exec_autocmds('User',{pattern='VeryLazy'})
local fixtures={rust={'main.rs','fn main() {}'},python={'main.py','print(1)'},typst={'main.typ','#let x = 1'},markdown={'main.md','# Heading'},config={'config.toml','[settings]'},pdf={'sample.pdf','%PDF-1.4'}}
for name,fixture in pairs(fixtures) do
  local file=root..'/'..fixture[1]
  vim.fn.writefile({fixture[2]},file)
  vim.cmd('edit '..vim.fn.fnameescape(file))
  vim.wait(2200,function() return #vim.lsp.get_clients({bufnr=0})>0 end,50)
  vim.api.nvim_exec_autocmds('InsertEnter',{buffer=0})
  vim.wait(500)
  snapshot(name)
  local logs={}
  for _,cmd in ipairs({'map','nmap','imap','vmap','xmap','omap','tmap'}) do
    local ok,result=pcall(vim.api.nvim_exec2,'verbose '..cmd,{output=true})
    logs[#logs+1]='## '..cmd..'\n'..(ok and result.output or tostring(result))
  end
  vim.fn.writefile(logs,root..'/verbose-'..name..'.txt')
end
local plugins={}
for name,p in pairs(require('lazy.core.config').plugins) do
  plugins[name]={dir=p.dir,lazy=p.lazy,loaded=p._ and p._.loaded~=nil,enabled=p.enabled,keys=vim.inspect(p.keys)}
end
vim.fn.writefile({vim.json.encode({snapshots=snapshots,plugins=plugins,version=vim.version(),leader=vim.g.mapleader,localleader=vim.g.maplocalleader})},root..'/runtime.json')
vim.cmd('qa!')
