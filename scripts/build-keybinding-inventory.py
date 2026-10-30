#!/usr/bin/env python3
"""Build evidence-preserving inventory from audit-nvim-keybindings.lua output."""
import collections, json, re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
raw=Path('/tmp/leafview-nvim-audit/runtime.json').read_bytes().decode('utf-8','replace')
runtime=json.loads(raw)
tui_path=Path('/tmp/leafview-nvim-audit-tui/runtime.json')
if tui_path.exists():
 tui=json.loads(tui_path.read_bytes().decode('utf-8','replace'))
 for sample in tui['snapshots']:
  sample['name']='tui:'+sample['name']
 runtime['snapshots'].extend(tui['snapshots'])
index=json.loads(Path('/tmp/leafview-nvim-source-index.json').read_text())
# Mapping is intentionally conservative: no nonexistent LSP/Git/terminal handlers.
equivalent={'gd':'lsp.definition','K':'lsp.hover','gK':'lsp.signature',' lR':'lsp.references',' lh':'lsp.signature',' lf':'lsp.format',' ld':'lsp.diagnostics',' w':'editor.save',' c':'editor.close',']b':'editor.next_buffer','[b':'editor.previous_buffer',' bp':'editor.previous_buffer',' e':'explorer.toggle',' ff':'files.find',' fr':'files.recent','<F1>':'lsp.hover','<F12>':'lsp.definition'}
entries={}
for sample in runtime['snapshots']:
 for scope in ['global','buffer']:
  for mode,maps in sample[scope].items():
   for m in maps:
    lhs=m['lhs'];source=m.get('callback_source') or m.get('script') or 'Neovim runtime/lazy handler'
    source=source.lstrip('@');plugin='user' if '/.config/nvim/' in source else source.split('/lazy/')[-1].split('/')[0] if '/lazy/' in source else 'Neovim'
    category='User' if plugin=='user' else 'AstroNvim' if plugin in ['AstroNvim','astrocore','astrolsp','astroui'] else 'Standard Neovim' if plugin=='Neovim' else 'Plugin'
    key=(scope,mode,lhs,source,m.get('rhs',''),m.get('desc',''),m.get('callback_line',0))
    if key not in entries:
     entries[key]={'keys':lhs.replace(' ','Space ',1) if lhs.startswith(' ') else lhs,'raw_lhs':lhs,'mode':mode,'action':m.get('rhs') or f'Lua callback {source}:{m.get("callback_line",0)}','description':m.get('desc',''),'category':category,'source_file':source,'source_line':m.get('callback_line',m.get('lnum',0)),'plugin':plugin,'scope':scope,'priority':100 if scope=='buffer' else 0,'activity':'observed','contexts':[],'conflict':False,'leafview_command':equivalent.get(lhs),'portable':lhs in equivalent,'notes':[]}
    entries[key]['contexts'].append(sample['name'])
values=list(entries.values())
groups=collections.defaultdict(list)
for e in values:groups[(e['scope'],e['mode'],e['raw_lhs'])].append(e)
for group in groups.values():
 if len(group)>1:
  for e in group:e['conflict']=True;e['notes'].append('Context/source variants; not necessarily simultaneously conflicting')
# Builtin Vim grammar is not returned by nvim_get_keymap. Keep its provenance separate.
standard=['h','j','k','l','w','W','b','B','e','E','ge','gE','0','^','$','gg','G','H','M','L','f{char}','F{char}','t{char}','T{char}',';',',','%','{','}','(',')','d{motion}','c{motion}','y{motion}','>{motion}','<{motion}','={motion}','g~{motion}','gu{motion}','gU{motion}','iw','aw','i"','a"',"i'","a'",'i(','a(','i[','a[','i{','a{','ip','ap','i','a','I','A','o','O','Esc','v','V','Ctrl+v',':','/','?','u','Ctrl+r','p','P','.']
for lhs in standard:
 values.append({'keys':lhs,'raw_lhs':lhs,'mode':'grammar','action':'Vim built-in grammar','description':lhs,'category':'Standard Vim','source_file':'Neovim :help index / motion.txt / operator.txt','source_line':0,'plugin':'Vim builtin','scope':'builtin','priority':-1,'activity':'builtin (not an API mapping)','contexts':['Vim modes as documented'],'conflict':False,'leafview_command':None,'portable':False,'notes':['Existing LeafView Vim grammar supports a subset; not counted as runtime mapping or automatically verified equivalent.']})
output=root/'docs/keybindings';output.mkdir(parents=True,exist_ok=True)
summary={'runtime_unique':len(entries),'builtin_grammar_entries':len(standard),'observed_portable':sum(e['portable'] for e in entries.values()),'observed_unmapped':sum(not e['portable'] for e in entries.values()),'context_variant_groups':sum(len(g)>1 for g in groups.values()),'inactive_templates':[e['file'] for e in index if e['inactive_template']]}
data={'summary':summary,'version':runtime['version'],'leader':runtime['leader'],'localleader':runtime['localleader'],'source_index':index,'plugins':runtime['plugins'],'samples':[{'name':s['name'],'filetype':s['filetype'],'clients':s['clients']} for s in runtime['snapshots']],'bindings':values,'limitations':['Both headless and real PTY TUI runs: startup + VeryLazy + files + InsertEnter + LSP attach; no interactive terminal buffer, DAP session, OpenCode service or marimo session started.','PDF fixture is not a valid PDF; extension hooks only, not interactive PDF plugin behavior. Its retained Rust filetype/client is evidence, not PDF LSP support.','Visual Line/Block use x/v mapping namespaces, not independent nvim_get_keymap modes.','Select/operator/terminal maps collected; entering all dynamic plugin states is not proven.','lua/community.lua returns {}; no lua/community directory exists. Installed plugins are not evidence of active community imports.','lhsraw binary termcodes were decoded with replacement for JSON transport; human-readable lhs is preserved.']}
(output/'neovim-inventory.json').write_text(json.dumps(data,ensure_ascii=False,indent=2))
lines=['# Neovim keybinding inventory','', 'Generated from current installed configuration; remote was not fetched.','', '## Counts','',*['- '+k+': '+str(v) for k,v in summary.items()],'','## Verification limits','',*['- '+s for s in data['limitations']],'','## Runtime and builtin registry','','| Keys | Mode | Description | Plugin | Scope | Observed contexts | LeafView |','|---|---|---|---|---|---|---|']
for e in values:
 cells=[e['keys'],e['mode'],e['description'] or e['action'],e['plugin'],e['scope'],', '.join(e['contexts']),e['leafview_command'] or 'not mapped / verify grammar']
 lines.append('| '+' | '.join(str(c).replace('|','\\|').replace('\n',' ') for c in cells)+' |')
(output/'neovim-inventory.md').write_text('\n'.join(lines)+'\n')
print(json.dumps(summary,ensure_ascii=False,indent=2))
