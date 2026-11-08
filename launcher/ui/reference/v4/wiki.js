/* ── 图鉴：法环物品栏式。数据来自 assets/wiki_sample.js（window.WIKI），条目图片在 assets/wiki/ ──
   左边焦点分三区：0「显示」行（筛选）、1「查找」行、2 条目区（网格 / 缩略图列表 / 单行列表）；
   右边说明面板固定高度，正文溢出时出现细滚动条，R 展开「详细说明」。
   依赖 index.html 主脚本里的 $、A、esc、key、arrows、go、back、guard、openSys、openLords、hideLords、S、LS、ST、NOTES、lpt、VER */
const WD=window.WIKI||{version:'',categories:[]};
const WIMG=p=>p?'assets/wiki/'+p.replace(/^img\//,''):'';
const stat=(e,k)=>((e.stats||[]).find(s=>s.k===k)||{}).v;
const COLS=5, NEWM='<img class="nw" src="assets/newmark.png" alt="新">';
/* 各分类的界面配置：标签图标、条目区样式、「显示」行里的具体类别取自哪个字段、空分类的两行字 */
const WCFG={
  weapon:{ic:'wt_weapon',kind:'grid',grp:e=>stat(e,'类别')},
  talisman:{ic:'wt_talisman',kind:'grid',grp:e=>e.tag},
  spell:{ic:'wt_spell',kind:'grid',grp:e=>stat(e,'类别'),none:['暂无法术条目','整合包新增的法术会列在这里']},
  map:{ic:'wm_snowup',kind:'thumbs',grp:e=>e.sub},
  enemy:{ic:'wt_enemy',kind:'grid',grp:e=>e.group},
  mechanic:{ic:'wt_system',kind:'list',grp:()=>''},
};
/* 没有游戏内图片的条目：特异地形借法环地图标签的浮雕作示意，属性分配借启动器里的属性面板图标 */
const WFALL={'map-crater':'wm_volcano','map-mountaintop':'wm_snowup','map-rotted':'wm_rotten','map-noklateo':'wm_nokron','map-hollow':'wm_hollow','mech-stat-alloc':'fl_levelup'};
/* 夜王用和永夜王格子同一套游戏头像：设为永夜王的显示紫色永夜版 */
const wicon=e=>e.nighterIndex!=null?lpt(e.nighterIndex,S.lords.includes(e.nighterIndex)):WIMG(e.icon)||(WFALL[e.id]?A(WFALL[e.id]):'');
/* 「本版新增」与更新页的更新内容同一份清单（index.html 的 NOTES） */
const NEWIDS=new Set(NOTES.map(r=>r.wiki).filter(Boolean));
WD.categories.forEach(c=>(c.entries||[]).forEach(e=>e.isNew=NEWIDS.has(e.id)));
const CATS=WD.categories.filter(c=>WCFG[c.key]).map(c=>({...c,entries:c.entries||[]}));   // 没有条目的分类（法术）也保留标签
/* c 分类下标；z 焦点区；q 查找文字；f / sel 按分类记住筛选项与选中条目；from 记住从哪里跳来；det 详细说明；ovf 正文溢出 */
const WS={c:0,z:2,q:'',qOld:'',edit:false,from:null,f:{},sel:{},det:false,ovf:false,one:false};
const wtabrow=$('#wtabrow'), wrows=$('#wrows'), wbox=$('#wbox'), wdesc=$('#wdesc');
let WL=[];                               // 当前可见条目（筛选 + 查找之后）
const cat=()=>CATS[WS.c], cfg=()=>WCFG[cat().key];
const wcur=()=>WL.find(e=>e.id===WS.sel[cat().key])||WL[0];
const wci=()=>Math.max(0,WL.indexOf(wcur()));

/* 有新增条目的分类，标签上带同一个新物品标记 */
wtabrow.innerHTML=CATS.map((c,i)=>`<button class="tab" tabindex="-1" data-i="${i}"><img class="tc" src="${A('tab_cursor')}" alt=""><img class="ic" src="${A(WCFG[c.key].ic)}" alt=""><span class="tl">${esc(c.name)}${c.entries.some(e=>e.isNew)?'<img class="nb" src="assets/newmark.png" alt="有新增">':''}</span></button>`).join('');
wtabrow.addEventListener('click',e=>{const b=e.target.closest('.tab');if(b)wtab(+b.dataset.i)});
$('#wprev').onclick=()=>wtab(WS.c-1); $('#wnext').onclick=()=>wtab(WS.c+1);

/* 「显示」的取值：全部 / 本版新增（有新条目才有）/ 具体类别（不止一种才列） */
function wopts(c=cat()){const g=[...new Set(c.entries.map(WCFG[c.key].grp).filter(Boolean))];
  return ['全部',...(c.entries.some(e=>e.isNew)?['本版新增']:[]),...(g.length>1?g:[])]}
const hay=e=>[e.name,e.sub,...(e.stats||[]).map(s=>s.v)].join(' ').toLowerCase();
/* useF=false 时忽略「显示」，用来数别的分类里有几条相符 */
function wlist(c=cat(),useF=true){const o=wopts(c)[useF?(WS.f[c.key]||0):0],q=WS.q.trim().toLowerCase(),g=WCFG[c.key].grp;
  return c.entries.filter(e=>(o==='全部'||(o==='本版新增'?e.isNew:g(e)===o))&&(!q||hay(e).includes(q)))}
const elsewhere=()=>CATS.map((c,i)=>i===WS.c||!WS.q.trim()?0:wlist(c,false).length);

/* ── 绘制 ── */
function wrender(){$('#wname').textContent=cat().name;wrowsPaint();wbody()}
/* 两行只在切分类、改筛选时重绘；打字时不重绘，免得输入框失焦。「显示」只有一个取值时整行隐藏 */
function wrowsPaint(){const o=wopts(),k=WS.f[cat().key]||0;WS.one=o.length<2;$('#codex').classList.toggle('one',WS.one);if(WS.one&&WS.z===0)WS.z=1;
  wrows.innerHTML=`<li class="row" data-z="0"><span class="lb">显示</span>${arrows(esc(o[k]),k<=0,k>=o.length-1)}</li>`
    +`<li class="row${WS.edit?' edit':''}" data-z="1"><span class="lb">查找</span><span class="val"><input class="wq" id="wq" maxlength="16" placeholder="—" value="${esc(WS.q)}"${WS.edit?'':' readonly'} tabindex="-1" spellcheck="false" autocomplete="off"></span></li>`;
  wzmark()}
function wbody(){WL=wlist();const q=WS.q.trim();
  wtabrow.querySelectorAll('.tab').forEach((b,i)=>{b.classList.toggle('on',i===WS.c);b.classList.toggle('none',!!q&&i!==WS.c&&!wlist(CATS[i],false).length)});
  if(WS.z===2&&!WL.length)WS.z=1;
  wboxPaint();wdescPaint();wzmark();whints()}
function wzmark(){[...wrows.children].forEach(li=>li.classList.toggle('sel',+li.dataset.z===WS.z));wbox.classList.toggle('on',WS.z===2)}
function wboxPaint(){const c=cat(),kind=cfg().kind,cur=wcur(),q=WS.q.trim();wbox.scrollTop=0;
  if(!WL.length){let a,b;
    if(q){const n=elsewhere(),hit=CATS.filter((_,i)=>n[i]).map(x=>`${x.name} ${n[CATS.indexOf(x)]} 条`);
      a=`没有找到「${q}」`;b=hit.length?`${hit.join(' · ')}。${WS.edit?'按 Enter 结束输入就跳过去':'按 Q / E 或单击亮着的标签'}`:WS.edit?'换个字试试，按 Enter 结束输入':'换个字试试'}
    else if(!c.entries.length)[a,b]=cfg().none||['暂无条目',''];
    else{a='没有符合的条目';b='按 ← → 换一种显示'}
    wbox.innerHTML=`<div class="wnone">${esc(a)}<span>${esc(b)}</span></div>`;return wfade()}
  /* 网格：可见的几行用淡空格补满，条目少的分类也是一格格槽位 */
  if(kind==='grid'){const n=Math.max(Math.ceil(WL.length/COLS),Math.floor(wbox.clientHeight/103))*COLS-WL.length;
    wbox.innerHTML=`<div class="wgrid">${WL.map((e,i)=>`<button class="wcell${e===cur?' f':''}" tabindex="-1" data-i="${i}" aria-label="${esc(e.name)}"><img class="cur" src="${A('sl_cursor')}" alt=""><img class="i${c.key==='enemy'?' pt':''}" src="${wicon(e)}" alt="">${e.isNew?NEWM:''}</button>`).join('')}${'<i class="wcell e"></i>'.repeat(n)}</div>`}
  else wbox.innerHTML=`<ul class="wl">${WL.map((e,i)=>kind==='thumbs'
    ?`<li class="row wth${e===cur?' sel':''}" data-i="${i}"><span class="tb"><img${e.icon?' class="col"':''} src="${wicon(e)}" alt=""></span><span class="tx"><span class="lb">${esc(e.name)}</span><span class="ln">${esc(e.desc)}</span></span>${e.isNew?NEWM:''}</li>`
    :`<li class="row${e===cur?' sel':''}" data-i="${i}"><img class="ic" src="${wicon(e)}" alt=""><span class="lb">${esc(e.name)}<span class="kd">${esc(e.sub)}</span></span>${e.isNew?NEWM:''}</li>`).join('')}</ul>`;
  wscroll()}
/* 光标所在条目保持在可见范围内；下面还有内容时底边渐隐 */
function wscroll(){const el=wbox.querySelector('.wcell.f,.row.sel');
  if(el){const t=el.offsetTop,b=t+el.offsetHeight,h=wbox.clientHeight;
    if(t<wbox.scrollTop+12)wbox.scrollTop=Math.max(0,t-12);else if(b>wbox.scrollTop+h-24)wbox.scrollTop=b-h+24}
  wfade()}
function wfade(){wbox.classList.toggle('dn',wbox.scrollTop+wbox.clientHeight<wbox.scrollHeight-1)}
wbox.addEventListener('scroll',wfade);

/* 说明面板：头像、名称、类别行、数值（短的两两并排）、正文、lore、获取方式；手写 / 占位文字暗色斜体，底部注明「示例文字」 */
function wdescPaint(){const e=WL.length?wcur():null;wdesc.className='desc wk'+(WS.det&&e?' det':'');
  if(!e){const q=WS.q.trim(),c=cat();WS.ovf=false;
    wdesc.innerHTML=`<div class="dh"><div class="pic box"><img src="${A(cfg().ic)}" alt=""></div><div><h2>${esc(c.name)}</h2>${q?`<p class="meta">查找「${esc(q)}」</p>`:''}</div></div><i class="sep"></i><div class="wbody"><p>${
      q?(WS.edit?'按 Enter 结束输入，会跳到第一个有结果的分类。':'按 Q / E 换到别的分类，有结果的标签是亮的。'):c.entries.length?'按 ← → 换一种显示。':'整合包新增的法术会列在这一类里。'}</p></div>`;return}
  const real=e.real||{}, ph=/^（占位）/.test(e.source||'')||!!e.srcH, lord=e.nighterIndex!=null, on=lord&&S.lords.includes(e.nighterIndex);
  const tg=[e.isNew?`${NEWM}${esc(WD.version)} 新增${VER!==WD.version?' · 更新后可用':''}`:'',e.tag&&!String(e.sub).includes(e.tag)?esc(e.tag):''].filter(Boolean).join(' · ');
  const head=`<div class="dh"><div class="pic ${e.group?'rd':'box'}"><img src="${wicon(e)}" alt=""></div><div><h2>${esc(e.name)}</h2><p class="meta">${esc(e.sub)}</p>${tg?`<p class="wtag">${tg}</p>`:''}</div></div>`;
  /* 武器的说明正文就是 lore 第一行：拆开显示，不重复 */
  const lore=String(e.lore||''), dup=e.desc&&lore.startsWith(e.desc), rest=(dup?lore.slice(e.desc.length):lore).trim();
  const kv=(e.stats||[]).filter(s=>!(lord&&s.k==='永夜王形态')), wide=s=>s.v.length>9||s.k.length>5;
  const notes=[real.name===false?'整条为手写':real.desc===false?'简介为手写':'',kv.some(s=>s.h)?'有利属性为推断':'',ph?(e.srcH?'来源说明为手写':'获取方式待补'):'',!e.icon&&WFALL[e.id]?'图为示意':''].filter(Boolean);
  wdesc.innerHTML=head+'<i class="sep"></i><div class="wbody">'
    +(lord?`<p class="ever"><img src="${lpt(e.nighterIndex,on)}" alt="">${on?`当前以永夜王形态出现：${esc(e.everdark.name)}`:'当前以普通形态出现'}</p>`:'')
    +(kv.length?`<dl class="kv">${kv.map(s=>`<div${wide(s)?' class="w"':''}><dt>${esc(s.k)}</dt><dd${s.h?' class="ph"':''}>${esc(s.v)}</dd></div>`).join('')}</dl>`:'')
    +(e.desc?`<p${real.desc===false?' class="ph"':''}>${esc(e.desc)}</p>`:'')+(rest?`<p class="lore">${esc(rest)}</p>`:'')+'</div><div class="wsb" hidden><i></i></div>'
    +`<div class="wfoot"><p class="wsrc"><b>${{map:'出现',enemy:'出现',mechanic:'来源'}[cat().key]||'获取方式'}</b>${ph?`<span class="ph">${esc(String(e.source).replace(/^（占位）/,''))}</span>`:esc(e.source||'—')}</p>`
    +(notes.length?`<p class="dt ph">示例文字：${notes.join('，')}，非游戏数据</p>`:'')+'</div>';
  const wb=wdesc.querySelector('.wbody'),sb=wdesc.querySelector('.wsb');
  /* 法环 scroll_waku 细滚动条：只在正文溢出时出现，跟着滚动 */
  const fd=()=>{const H=wb.scrollHeight,h=wb.clientHeight,more=H>h+1;wb.classList.toggle('dn',more&&wb.scrollTop+h<H-1);sb.hidden=!more;if(!more)return;
    const th=Math.max(28,h*h/H);sb.style.top=wb.offsetTop+'px';sb.style.height=h+'px';sb.firstChild.style.height=th+'px';sb.firstChild.style.transform=`translateY(${wb.scrollTop*(h-th)/(H-h)}px)`};
  wb.addEventListener('scroll',fd);fd();if(!WS.det)WS.ovf=wb.scrollHeight>wb.clientHeight+1}

/* 底部按键提示随焦点区变化，能执行的都可以点；正文溢出时才出现「详细说明」 */
function whints(){const e=WL.length?wcur():null;let h;
  if(WS.edit)h=key('key_enter','完成','Enter')+key('key_esc','取消','Escape');
  else if(WS.det)h=key('key_ud','滚动')+key('key_r','收起','r');
  else{h=key(WS.z===2&&cfg().kind==='grid'?'key_udlr':'key_ud','选择');
    if(WS.z===0)h+=key('key_lr','显示');
    if(WS.z===1){h+=key('key_enter','输入','Enter');if(WS.q)h+=key('key_bs','清空','Backspace')}
    if(WS.z===2&&e&&e.nighterIndex!=null)h+=key('key_enter','永夜王设置','Enter');
    if(e&&WS.ovf)h+=key('key_r','详细说明','r');
    h+=key('key_esc','返回','Escape')}
  $('#ckeys').innerHTML=h}

/* ── 操作 ── */
function wtab(i){if(WS.edit)wedit(false);WS.det=false;WS.c=(i+CATS.length)%CATS.length;wrender()}
function wzone(z){WS.z=z===2&&!WL.length?1:z===0&&WS.one?1:z;wzmark();whints();if(WS.z===2)wscroll()}
/* mouse=true（悬停选中）时不自动滚动，免得网格在静止的鼠标下移动 */
function wpick(j,mouse){if(!WL[j])return;WS.sel[cat().key]=WL[j].id;const grid=cfg().kind==='grid';
  wbox.querySelectorAll(grid?'.wcell[data-i]':'.row').forEach((el,k)=>el.classList.toggle(grid?'f':'sel',k===j));if(mouse)wfade();else wscroll();wdescPaint();whints()}
function wfilter(d){const o=wopts(),k=WS.f[cat().key]||0,n=Math.max(0,Math.min(o.length-1,k+d));if(n===k)return;WS.f[cat().key]=n;wrowsPaint();wbody()}
function wsetq(q){WS.q=q;const i=$('#wq');if(i)i.value=q;wbody()}
function wdet(on){WS.det=on&&WS.ovf;wdescPaint();whints()}
/* 查找：Enter 开始输入；再按 Enter 完成并把光标移到结果上，Esc 取消这次输入 */
function wedit(on,cancel){const inp=$('#wq');if(!inp)return;const li=inp.closest('.row');
  if(on){WS.edit=true;WS.qOld=WS.q;WS.z=1;inp.readOnly=false;li.classList.add('edit');inp.focus();inp.setSelectionRange(inp.value.length,inp.value.length)}
  else{if(!WS.edit)return;WS.edit=false;inp.readOnly=true;li.classList.remove('edit');inp.blur();if(cancel&&WS.q!==WS.qOld)wsetq(WS.qOld)}
  wzmark();whints()}
/* 结束输入时当前分类没有结果：跳到第一个有结果的分类（「显示」重置为全部） */
function wjump(){if(WL.length||!WS.q.trim())return;const j=elsewhere().findIndex(x=>x>0);if(j<0)return;WS.c=j;WS.f[cat().key]=0;wrender()}
wrows.addEventListener('input',e=>{if(e.target.id==='wq'){WS.q=e.target.value;wbody()}});
wrows.addEventListener('focusout',e=>{if(e.target.id==='wq'&&WS.edit){wedit(false);wjump()}});   // 左键点到别处即完成输入（右键不抢焦点）

function wkey(k){
  if(WS.edit){if(k==='Enter'){wedit(false);wjump();wzone(2)}else if(k==='Escape')wedit(false,true);else if(k==='ArrowDown'||k==='ArrowUp'){wedit(false);wzone(k==='ArrowDown'?2:0)}else return false;return true}
  /* 详细说明：↑↓ 滚动正文，R / Esc 收起 */
  if(WS.det){const wb=wdesc.querySelector('.wbody');
    if(k==='ArrowDown'||k==='ArrowUp')wb.scrollBy({top:k==='ArrowDown'?72:-72,behavior:'smooth'});else if(k==='r'||k==='R'||k==='Escape')wdet(false);else return false;return true}
  const grid=cfg().kind==='grid',cols=grid?COLS:1,n=WL.length,i=wci();
  if(k==='Escape')wback();
  else if(k==='q'||k==='Q')wtab(WS.c-1);else if(k==='e'||k==='E')wtab(WS.c+1);
  else if((k==='r'||k==='R')&&WS.ovf&&n)wdet(true);
  else if(WS.z===0){if(k==='ArrowLeft'||k==='ArrowRight')wfilter(k==='ArrowLeft'?-1:1);else if(k==='ArrowDown')wzone(1);else if(k!=='ArrowUp')return false}
  else if(WS.z===1){if(k==='ArrowUp')wzone(0);else if(k==='ArrowDown')wzone(2);else if(k==='Enter')wedit(true);else if(k==='Backspace'||k==='Delete'){if(WS.q)wsetq('')}else if(!k.startsWith('Arrow'))return false}
  else if(k.startsWith('Arrow')){
    if(k==='ArrowUp'){if(i-cols<0)wzone(1);else wpick(i-cols)}
    else if(k==='ArrowDown'){if(i+cols<n)wpick(i+cols);else if(Math.floor(i/cols)<Math.floor((n-1)/cols))wpick(n-1)}   // 下一行不满时落到最后一格
    else if(grid){const j=i+(k==='ArrowLeft'?-1:1);if(j>=0&&j<n&&Math.floor(j/cols)===Math.floor(i/cols))wpick(j)}}   // 左右不换行
  else if(k==='Enter'){const e=wcur();if(!e||e.nighterIndex==null)return false;wlord(e.nighterIndex)}
  else return false;
  return true}

/* 鼠标：移动指针即选中（打字时不抢焦点），单击只选中；「显示」行点箭头改值，「查找」行单击开始输入 */
function whover(t){if(WS.edit)return;const c=t.closest('#wbox .wcell[data-i],#wbox .row'),r=t.closest('#wrows .row');
  if(c){const j=+c.dataset.i;if(WS.z!==2)wzone(2);if(WL[j]&&WL[j]!==wcur())wpick(j,true)}
  else if(r&&+r.dataset.z!==WS.z)wzone(+r.dataset.z)}
wbox.addEventListener('click',e=>{const c=e.target.closest('.wcell[data-i],.row');if(!c||e.detail>1)return;wzone(2);wpick(+c.dataset.i,true)});
wrows.addEventListener('click',e=>{const r=e.target.closest('.row');if(!r)return;const z=+r.dataset.z,ar=e.target.closest('.ar');
  if(z===0){wzone(0);if(ar)wfilter(+ar.dataset.d)}else if(!WS.edit)wedit(true)});

/* ── 跳转：首页 / 更新页 / 永夜王选择 ⇄ 图鉴；from 只记一层，来回跳不会越叠越深 ── */
function openWiki(id,from){if(WS.edit)wedit(false);WS.from=from||null;WS.det=false;
  if(!id&&!from)WS.q='';                                         // 从首页进来：清空上次的查找
  const ci=id?CATS.findIndex(c=>c.entries.some(e=>e.id===id)):-1;
  if(ci>=0){WS.c=ci;const k=CATS[ci].key;WS.f[k]=0;WS.q='';WS.sel[k]=id;WS.z=2}
  wrender();go('codex')}
function wresume(){wrender();go('codex')}                       // 从永夜王选择回来：图鉴保持原样，只刷新永夜状态
function wback(){if(WS.edit)wedit(false);const f=WS.from;WS.from=null;
  if(!f)return back();
  if(f.k==='lords'){openSys('night',2);openLords(f.i);return}
  openSys(f.k,f.row)}
/* 图鉴 → 永夜王：若本来就是从永夜王选择跳来的，直接回去并选中这位夜王 */
function wlord(i){if(WS.from&&WS.from.k==='lords'){WS.from.i=i;return wback()}
  openSys('night',2);openLords(i);LS.from='wiki'}
/* 永夜王 → 图鉴（F）：若本来就是从图鉴跳来的，直接回图鉴 */
function lordWiki(){const id='nightlord-'+LS.f;
  if(LS.from==='wiki'){WS.c=Math.max(0,CATS.findIndex(c=>c.key==='enemy'));WS.f.enemy=0;WS.q='';WS.sel.enemy=id;WS.z=2;return closeLords()}
  hideLords();openWiki(id,{k:'lords',i:LS.f})}

/* ── ?state= 直达（截图用） ── */
Object.assign(ST,{
  'wiki-weapons':()=>openWiki('weapon-89000000'),
  'wiki-talismans':()=>openWiki('talisman-20015'),
  'wiki-magic':()=>{openWiki();wtab(CATS.findIndex(c=>c.key==='spell'))},
  'wiki-map':()=>openWiki('map-crater'),
  'wiki-enemies':()=>openWiki('nightlord-0'),
  'wiki-detail':()=>{openWiki('nightlord-0');wdet(true)},
  'wiki-system':()=>openWiki('mech-stat-alloc'),
  'wiki-search':()=>{openWiki('weapon-4600000');wsetq('大剑');wedit(true)},
  'wiki-search-none':()=>{openWiki('weapon-89000000');wedit(true);wsetq('卡莉果')},
});
