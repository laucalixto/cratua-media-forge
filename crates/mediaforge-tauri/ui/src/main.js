// ── Cratua Media Forge — Frontend ──
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, confirm } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';

// ── State ──
const S={mode:'simple',presets:[],selectedPreset:'default',_syncingPreset:false,params:{video_codec:'H264',width:1920,height:1080,scale_algorithm:'Lanczos',fps:'SameAsSource',crf:19,video_bitrate:null,max_bitrate:null,bufsize:null,preset:'Medium',profile:null,pixel_format:'Yuv420p',deinterlace:null,video_filters:[],audio_codec:'Aac',audio_bitrate:128,audio_channels:2,sample_rate:48000,audio_filters:[],container:'Mp4',movflags:['FastStart'],threads:0,metadata:{},trim_start:null,trim_end:null,extra_args:[]},files:[],outputDir:'',jobs:[],isEncoding:false,config:null,history:[]};

const $=s=>document.querySelector(s),$$=s=>document.querySelectorAll(s),on=(el,ev,fn)=>el.addEventListener(ev,fn);
function normPath(p){return(p||'').replace(/\\/g,'/')}
function parseVideoFilter(s){if(s==='HFlip')return{HFlip:null};if(s==='VFlip')return{VFlip:null};if(s==='Denoise')return{Denoise:null};if(s==='Grayscale')return{Grayscale:null};const m=s.match(/^Rotate\((\d+)\)$/);if(m)return{Rotate:parseInt(m[1])};const b=s.match(/^Brightness\(([0-9.+\-]+)\)$/);if(b)return{Brightness:parseFloat(b[1])};const c=s.match(/^Contrast\(([0-9.+\-]+)\)$/);if(c)return{Contrast:parseFloat(c[1])};const st=s.match(/^Saturation\(([0-9.+\-]+)\)$/);if(st)return{Saturation:parseFloat(st[1])};return{HFlip:null}}
function parseAudioFilter(s){if(s==='Loudnorm')return{Loudnorm:null};const m=s.match(/^Volume\(([0-9.]+)\)$/);if(m)return{Volume:parseFloat(m[1])};const h=s.match(/^Highpass\((\d+)\)$/);if(h)return{Highpass:parseInt(h[1])};const l=s.match(/^Lowpass\((\d+)\)$/);if(l)return{Lowpass:parseInt(l[1])};return{Loudnorm:null}}

// ── Diag + right-click ──
const diagEl=document.createElement('div');diagEl.style.cssText='position:fixed;bottom:0;left:0;right:0;background:#111;color:#0f0;font:10px monospace;padding:4px 10px;z-index:999;opacity:0.9';document.body.appendChild(diagEl);
function diag(m){diagEl.textContent=m;console.log(m)}
document.addEventListener('contextmenu',e=>e.preventDefault());

// ── Splash ──
setTimeout(()=>{const s=$('#splash');s.style.opacity='0';s.style.transition='opacity 0.5s';setTimeout(()=>{s.classList.add('hidden');$('#app').classList.remove('hidden')},500)},2000);

// ── Mode ──
function setMode(m){S.mode=m;$('#btn-simple').className=m==='simple'?'px-3 py-1.5 text-xs font-medium rounded-md bg-rose text-white':'px-3 py-1.5 text-xs font-medium rounded-md text-[#9090a0] hover:text-white hover:bg-[#1f1f2e]';$('#btn-advanced').className=m==='advanced'?'px-3 py-1.5 text-xs font-medium rounded-md bg-rose text-white':'px-3 py-1.5 text-xs font-medium rounded-md text-[#9090a0] hover:text-white hover:bg-[#1f1f2e]';$('#mode-simple').classList.toggle('hidden',m!=='simple');$('#mode-advanced').classList.toggle('hidden',m!=='advanced');
if(m==='advanced')syncSimpleToAdvanced()}
on($('#btn-simple'),'click',()=>setMode('simple'));on($('#btn-advanced'),'click',()=>setMode('advanced'));

// ── Sync Simple → Advanced ──
function syncSimpleToAdvanced(){
  $('#a-width').value=S.params.width;$('#a-height').value=S.params.height;
  $('#a-crf').value=S.params.crf??23;$('#a-crf-num').value=S.params.crf??23;$('#a-crf-val').textContent=S.params.crf??23;
  updateCrfWarning();updateProfileOptions()
}

// ── Tabs ──
$$('.tab-btn').forEach(b=>on(b,'click',()=>{$$('.tab-btn').forEach(x=>x.classList.remove('active'));b.classList.add('active');$$('.tab-panel').forEach(x=>x.classList.add('hidden'));$(`#tab-${b.dataset.tab}`).classList.remove('hidden');
if(b.dataset.tab==='output')updateCmdPreview()}));

// ── Command Preview: update on any param change ──
function hookCmdPreview(){
  const ids=['#a-vcodec','#a-acodec','#a-container','#a-vbitrate','#a-maxbr','#a-bufsize',
    '#a-preset-speed','#a-profile','#a-pixfmt','#a-width','#a-height','#a-scale','#a-fps-mode','#a-fps-val',
    '#a-deint','#a-deint-method','#a-threads','#a-channels','#a-samplerate',
    '#a-faststart','#a-fragkf','#a-trim-start','#a-trim-end','#a-extra-args',
    '#a-crf','#a-crf-num','#a-abitrate','#a-abitrate-num','#a-deint','#a-faststart','#a-fragkf',
    '#vfilter-add','#afilter-add'];
  ids.forEach(id=>{const el=$(id);if(el){on(el,'change',()=>{updateCmdPreview();if(!S._syncingPreset)markPresetCustom()});on(el,'input',()=>{updateCmdPreview();if(!S._syncingPreset)markPresetCustom()})}});
}

// ── Collapsible (▶/▼ toggle) ──
function setCollapsible(h){const arrow=h.querySelector('.arrow');const body=h.nextElementSibling;const isOpen=h.classList.contains('open');body.style.display=isOpen?'':'none';if(arrow)arrow.textContent=isOpen?'▼':'▶'}$$('.collapsible-header').forEach(h=>{setCollapsible(h);on(h,'click',()=>{h.classList.toggle('open');setCollapsible(h);if(h.classList.contains('open')&&h.textContent.includes('Command Preview'))updateCmdPreview()})});

// ── FPS ──
on($('#a-fps-mode'),'change',e=>{$('#a-fps-val').classList.toggle('hidden',e.target.value!=='fixed')});

// ── Sliders ──
function bindSN(rid,nid,vid,suf){const r=$(rid),n=$(nid),v=vid?$(vid):null;if(!r||!n)return;on(r,'input',()=>{n.value=r.value;if(v)v.textContent=suf?r.value+suf:r.value});on(n,'change',()=>{let x=parseInt(n.value);if(isNaN(x))return;x=Math.max(parseInt(r.min),Math.min(parseInt(r.max),x));r.value=x;n.value=x;if(v)v.textContent=suf?x+suf:x;if(rid==='#a-crf')updateCrfWarning()})}
bindSN('#s-crf','#s-crf-num','#s-crf-val','');bindSN('#s-audio','#s-audio-num','#s-audio-val',' kbps');
bindSN('#a-crf','#a-crf-num','#a-crf-val','');bindSN('#a-abitrate','#a-abitrate-num','#a-abitrate-val',' kbps');
on($('#a-crf-auto'),'click',()=>{$('#a-crf').value=23;$('#a-crf-num').value=23;$('#a-crf-val').textContent='23';updateCrfWarning()});

// ── CRF lossless warning ──
function updateCrfWarning(){const v=parseInt($('#a-crf').value);$('#a-crf-val').textContent=v===0?'lossless ⚠':v}

// ── Profile dinâmico ──
const profilesByCodec={H264:['','baseline','main','high'],H265:['','main','main10','main12'],VP9:[],AV1:[],Copy:[],SVTAV1:[]};
function updateProfileOptions(){const codec=$('#a-vcodec').value,profs=profilesByCodec[codec]||[],sel=$('#a-profile'),cur=sel.value;sel.innerHTML=profs.length?profs.map(p=>`<option value="${p}">${p||'auto'}</option>`).join(''):'<option value="">auto</option>';if(profs.includes(cur))sel.value=cur;updatePixelWarn()}
on($('#a-vcodec'),'change',()=>{updateProfileOptions();autoContainer()});
on($('#a-profile'),'change',updatePixelWarn);
on($('#a-pixfmt'),'change',updatePixelWarn);

// ── Pixel + Profile conflito ──
function updatePixelWarn(){const pix=$('#a-pixfmt').value,prof=$('#a-profile').value,el=$('#pixel-warn');if(!el)return;const bad=(pix==='yuv444p'||pix==='rgb24')&&(prof==='baseline');el.style.display=bad?'block':'none';if(bad)el.textContent='⚠ This pixel format requires profile main or high'}

// ── Auto container ──
function autoContainer(){const codec=$('#a-vcodec').value;if(codec==='VP9')$('#a-container').value='Webm';if(codec==='Copy')$('#a-container').value='Mp4'}function fixOpusSampleRate(){const ac=$('#a-acodec').value;const sr=parseInt($('#a-samplerate').value);if(ac==='Opus'&&![48000,24000,16000,12000,8000].includes(sr)){$('#a-samplerate').value='48000'}}function onContainerChange(){const cont=$('#a-container').value;if(cont==='Webm'){$('#a-vcodec').value='VP9';$('#a-acodec').value='Opus';$('#a-faststart').checked=false;$('#a-fragkf').checked=false;fixOpusSampleRate()}else if(cont==='Gif'){$('#a-vcodec').value='H264';$('#a-fps-mode').value='fixed';$('#a-fps-val').classList.remove('hidden');$('#a-fps-val').value=10;$('#a-faststart').checked=false;$('#a-fragkf').checked=false}}
on($('#a-acodec'),'change',()=>{fixOpusSampleRate();updateCmdPreview()});
on($('#a-container'),'change',()=>{onContainerChange();updateCmdPreview()});

// ── Filters ──
function vfLabel(f){if(typeof f==='string')return f;for(const k of['HFlip','VFlip','Denoise','Grayscale','Rotate','Brightness','Contrast','Saturation']){if(k in f){switch(k){case'HFlip':return'Flip H';case'VFlip':return'Flip V';case'Denoise':return'Denoise';case'Grayscale':return'Grayscale';case'Rotate':return'Rotate '+f.Rotate+'\u00b0';case'Brightness':return'Bright '+f.Brightness;case'Contrast':return'Contrast '+f.Contrast;case'Saturation':return'Sat '+f.Saturation}}}return JSON.stringify(f)}function afLabel(f){if(typeof f==='string')return f;for(const k of['Loudnorm','Volume','Highpass','Lowpass']){if(k in f){switch(k){case'Loudnorm':return'Loudnorm';case'Volume':return'Vol '+f.Volume+'x';case'Highpass':return'HP '+f.Highpass+'Hz';case'Lowpass':return'LP '+f.Lowpass+'Hz'}}}return JSON.stringify(f)}function renderFilters(){const vl=$('#vfilters-list');vl.innerHTML=S.params.video_filters.map((f,i)=>`<div class=\"flex items-center justify-between text-xs py-1 px-2 rounded hover:bg-[#1f1f2e]\"><span class=\"text-[#c0c0d0]\">${vfLabel(f)}</span><button data-vfidx=\"${i}\" class=\"text-[#606070] hover:text-[#ef4444]\">\u2715</button></div>`).join('');vl.querySelectorAll('button').forEach(b=>on(b,'click',()=>{S.params.video_filters.splice(parseInt(b.dataset.vfidx),1);renderFilters();updateCmdPreview()}));const al=$('#afilters-list');al.innerHTML=S.params.audio_filters.map((f,i)=>`<div class=\"flex items-center justify-between text-xs py-1 px-2 rounded hover:bg-[#1f1f2e]\"><span class=\"text-[#c0c0d0]\">${afLabel(f)}</span><button data-afidx=\"${i}\" class=\"text-[#606070] hover:text-[#ef4444]\">\u2715</button></div>`).join('');al.querySelectorAll('button').forEach(b=>on(b,'click',()=>{S.params.audio_filters.splice(parseInt(b.dataset.afidx),1);renderFilters();updateCmdPreview()}))}
on($('#vfilter-add'),'change',e=>{if(e.target.value){S.params.video_filters.push(parseVideoFilter(e.target.value));renderFilters();e.target.value=''}});
on($('#afilter-add'),'change',e=>{if(e.target.value){S.params.audio_filters.push(parseAudioFilter(e.target.value));renderFilters();e.target.value=''}});

// ── Metadata ──
function renderMetadata(){const l=$('#metadata-list');l.innerHTML=Object.entries(S.params.metadata).map(([k,v])=>`<div class="flex items-center justify-between py-1 px-2 rounded hover:bg-[#1f1f2e]"><span class="text-[#c0c0d0]">${k}: ${v}</span><button data-mkey="${k}" class="text-[#606070] hover:text-[#ef4444]">✕</button></div>`).join('');l.querySelectorAll('button').forEach(b=>on(b,'click',()=>{delete S.params.metadata[b.dataset.mkey];renderMetadata()}))}
on($('#meta-add'),'click',()=>{const k=$('#meta-key').value.trim(),v=$('#meta-val').value.trim();if(k){S.params.metadata[k]=v;renderMetadata();$('#meta-key').value='';$('#meta-val').value='';diag('Meta added: '+k)}else{diag('Meta: key empty')}});

// ── Command Preview ──
async function updateCmdPreview(){try{const p=S.mode==='simple'?collectSimpleParams():collectAdvParams();const ext=p.container?p.container.toLowerCase():'mp4';const cmd=await invoke('build_command_preview',{params:p});if($('#cmd-preview'))$('#cmd-preview').value=cmd.replace(/output\.mp4/g,'output.'+ext)}catch(e){}}

// ── Files ──
function addFiles(paths){if(!paths||!paths.length)return;const np=paths.map(normPath).filter(p=>!S.files.includes(p));if(np.length)S.files.push(...np);renderFiles();updateStatus()}
function renderFiles(){const l=$('#file-list');if(!S.files.length){l.innerHTML='<div class="text-[#606070] text-xs p-2 text-center">Drop files or click + Add Files</div>'}else{l.innerHTML=S.files.map((f,i)=>`<div class="flex items-center justify-between text-xs py-1 px-2 rounded hover:bg-[#1f1f2e]"><span class="truncate text-[#c0c0d0]" title="${f}">${f.split('/').pop()}</span><button data-fidx="${i}" class="text-[#606070] hover:text-[#ef4444]">✕</button></div>`).join('');l.querySelectorAll('button').forEach(b=>on(b,'click',()=>{S.files.splice(parseInt(b.dataset.fidx),1);renderFiles();updateStatus()}))}}
on($('#btn-add-files'),'click',async()=>{try{const sel=await dialogOpen({multiple:true,filters:[{name:'Media',extensions:['mp4','mkv','mov','avi','webm','mp3','wav','flac','m4a','ogg']}]});if(sel)addFiles(Array.isArray(sel)?sel:[sel])}catch(e){diag('Dialog error: '+e)}});
on($('#btn-browse'),'click',async()=>{try{const sel=await dialogOpen({directory:true,multiple:false,title:'Select Output Folder'});if(sel){S.outputDir=normPath(typeof sel==='string'?sel:sel[0]);updateOutputDisplay();if(S.config){S.config.output_dir=S.outputDir;invoke('save_config',{config:S.config}).catch(()=>{})}}}catch(e){diag('Browse error: '+e)}});
// Advanced browse
on($('#a-btn-browse'),'click',async()=>{try{const sel=await dialogOpen({directory:true,multiple:false,title:'Select Output Folder'});if(sel){S.outputDir=normPath(typeof sel==='string'?sel:sel[0]);updateOutputDisplay();if(S.config){S.config.output_dir=S.outputDir;invoke('save_config',{config:S.config}).catch(()=>{})}}}catch(e){diag('Browse error: '+e)}});

function updateOutputDisplay(){
  const d=S.outputDir||'~';
  const el=$('#output-dir');if(el)el.textContent=d;
  const ael=$('#a-output-dir');if(ael)ael.textContent=d;
}
on($('#path-input'),'keydown',e=>{if(e.key==='Enter'){addFiles([e.target.value.trim()]);e.target.value=''}});
on($('#btn-clear-files'),'click',()=>{S.files=[];renderFiles();updateStatus()});

// ── Drag-drop ──
async function setupDragDrop(){try{const wv=getCurrentWebview();await wv.onDragDropEvent(ev=>{if(ev.payload.type==='over')$('#file-list').classList.add('ring-2','ring-rose');else if(ev.payload.type==='leave')$('#file-list').classList.remove('ring-2','ring-rose');else if(ev.payload.type==='drop'){$('#file-list').classList.remove('ring-2','ring-rose');if(ev.payload.paths?.length)addFiles(ev.payload.paths)}})}catch(e){}}

function updateStatus(){$('#status-count').textContent=`${S.files.length} files | ${S.jobs.length} jobs`}

// ── Params ──
function even(n){n=parseInt(n);if(isNaN(n))return 1920;return n%2!==0?n+1:n}
function collectSimpleParams(){return{...S.params,width:parseInt($('#s-width').value)||1920,height:parseInt($('#s-height').value)||1080,crf:parseInt($('#s-crf').value)||19,audio_bitrate:parseInt($('#s-audio').value)||128,deinterlace:$('#s-deint').checked?'Yadif':null}}
function collectAdvParams(){
  const fps=$('#a-fps-mode').value==='fixed'?{Fixed:parseInt($('#a-fps-val').value)||30}:'SameAsSource';
  const sm={ultrafast:'Ultrafast',superfast:'Superfast',veryfast:'Veryfast',faster:'Faster',fast:'Fast',medium:'Medium',slow:'Slow',slower:'Slower',veryslow:'Veryslow'};
  const pm={yuv420p:'Yuv420p',yuv422p:'Yuv422p',yuv444p:'Yuv444p',nv12:'Nv12',rgb24:'Rgb24'};
  const mf=[];if($('#a-faststart').checked)mf.push('FastStart');if($('#a-fragkf').checked)mf.push('FragKeyframe');
  const ea=$('#a-extra-args').value.trim();
  // CRF prioritário: se CRF>0, ignora bitrate
  const crfVal=parseInt($('#a-crf').value);const crf=isNaN(crfVal)?null:crfVal;
  const vbr=crf&&crf>0?null:(parseInt($('#a-vbitrate').value)||null);
  const mbr=crf&&crf>0?null:(parseInt($('#a-maxbr').value)||null);
  const buf=crf&&crf>0?null:(parseInt($('#a-bufsize').value)||null);
  return{...S.params,
    video_codec:$('#a-vcodec').value||'H264',audio_codec:$('#a-acodec').value||'Aac',
    container:$('#a-container').value||'Mp4',crf,video_bitrate:vbr,max_bitrate:mbr,bufsize:buf,
    preset:sm[$('#a-preset-speed').value||'medium']||'Medium',
    profile:$('#a-profile').value?$('#a-profile').value.charAt(0).toUpperCase()+$('#a-profile').value.slice(1):null,
    pixel_format:pm[$('#a-pixfmt').value||'yuv420p']||'Yuv420p',
    deinterlace:$('#a-deint').checked?($('#a-deint-method').value==='bwdif'?'Bwdif':'Yadif'):null,
    fps,width:even($('#a-width').value),height:even($('#a-height').value),
    scale_algorithm:$('#a-scale').value||'Lanczos',
    audio_bitrate:parseInt($('#a-abitrate').value)||128,audio_channels:parseInt($('#a-channels').value)||2,
    sample_rate:parseInt($('#a-samplerate').value)||48000,threads:parseInt($('#a-threads').value)||0,
    movflags:mf,trim_start:$('#a-trim-start').value.trim()||null,trim_end:$('#a-trim-end').value.trim()||null,
    extra_args:ea?ea.split(/\s+/).filter(Boolean):[]
  }
}

// ── Preset selector ──
on($('#preset-select'),'change',()=>{const id=$('#preset-select').value,p=S.presets.find(x=>x.id===id);if(p&&p.params){S._syncingPreset=true;S.selectedPreset=id;S.params={...p.params};syncPresetToUI();S._syncingPreset=false}});
function syncPresetToUI(){
  // Helper to set value if element exists
  const sv=(id,v)=>{const el=$(id);if(el&&v!==undefined&&v!==null)el.value=v};
  // Values from S.params
  const p=S.params;
  // Simple mode fields
  sv('#s-width',p.width);sv('#s-height',p.height);
  sv('#s-crf',p.crf??19);const scn=$('#s-crf-num');if(scn)scn.value=p.crf??19;const scv=$('#s-crf-val');if(scv)scv.textContent=p.crf??19;
  sv('#s-audio',p.audio_bitrate);const san=$('#s-audio-num');if(san)san.value=p.audio_bitrate;const sav=$('#s-audio-val');if(sav)sav.textContent=(p.audio_bitrate||128)+' kbps';
  const sdi=$('#s-deint');if(sdi)sdi.checked=!!p.deinterlace;
  // Advanced mode fields
  sv('#a-width',p.width);sv('#a-height',p.height);
  sv('#a-crf',p.crf??19);const acn=$('#a-crf-num');if(acn)acn.value=p.crf??19;const acv=$('#a-crf-val');if(acv)acv.textContent=p.crf??19;
  sv('#a-abitrate',p.audio_bitrate);const aan=$('#a-abitrate-num');if(aan)aan.value=p.audio_bitrate;const aav=$('#a-abitrate-val');if(aav)aav.textContent=(p.audio_bitrate||128)+' kbps';
  const adi=$('#a-deint');if(adi)adi.checked=!!p.deinterlace;
  sv('#a-deint-method',p.deinterlace==='Bwdif'?'bwdif':'yadif');
  // Video
  sv('#a-vcodec',p.video_codec);
  sv('#a-container',p.container);
  sv('#a-pixfmt',p.pixel_format?p.pixel_format.toLowerCase():'yuv420p');
  sv('#a-preset-speed',p.preset?p.preset.toLowerCase():'medium');
  sv('#a-scale',p.scale_algorithm||'Lanczos');
  sv('#a-threads',p.threads??0);
  // FPS
  if(p.fps==='SameAsSource'){sv('#a-fps-mode','same');$('#a-fps-val').classList.add('hidden')}
  else if(p.fps&&typeof p.fps==='object'&&'Fixed' in p.fps){sv('#a-fps-mode','fixed');sv('#a-fps-val',p.fps.Fixed);$('#a-fps-val').classList.remove('hidden')}
  else{sv('#a-fps-mode','same');$('#a-fps-val').classList.add('hidden')}
  // Audio
  sv('#a-acodec',p.audio_codec||'Aac');
  sv('#a-samplerate',p.sample_rate||48000);
  sv('#a-channels',p.audio_channels??2);
  // Movflags
  const mf=p.movflags||[];
  const fs=$('#a-faststart');if(fs)fs.checked=mf.includes('FastStart');
  const fk=$('#a-fragkf');if(fk)fk.checked=mf.includes('FragKeyframe');
  // Trim
  sv('#a-trim-start',p.trim_start||'');
  sv('#a-trim-end',p.trim_end||'');
  // Extra args
  sv('#a-extra-args',(p.extra_args||[]).join(' '));
  // Profile
  sv('#a-profile',p.profile?p.profile.toLowerCase():'');
  // Side effects
  updateProfileOptions();autoContainer();renderFilters();renderMetadata();updateCrfWarning();
  // Update selectedPreset to match
  S.selectedPreset=$('#adv-preset-select').value||$('#preset-select').value||'default';
}
// Mark preset as custom when user changes any field
function markPresetCustom(){
  S.selectedPreset='';
  ['#preset-select','#adv-preset-select'].forEach(sid=>{const sel=$(sid);if(sel&&sel.value!=='')sel.value=''})
}

// ── Save / Delete preset ──
on($('#adv-preset-select'),'change',()=>{const id=$('#adv-preset-select').value,p=S.presets.find(x=>x.id===id);if(p&&p.params){S._syncingPreset=true;S.selectedPreset=id;S.params={...p.params};syncPresetToUI();S._syncingPreset=false}});
on($('#adv-btn-save-preset'),'click',async()=>{const name=prompt('Preset name:');if(!name)return;const params=collectAdvParams();const id=name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');const ex=S.presets.find(p=>p.id===id);if(ex){if(!await confirm('Preset "'+ex.name+'" already exists. Overwrite?'))return}try{await invoke('create_preset',{preset:{id,name,description:'Custom preset',category:'Video',params}});S.presets=await invoke('get_presets');populatePresetDropdowns(id);diag('Preset saved: '+name)}catch(e){diag('Save error: '+e)}});
on($('#adv-btn-delete-preset'),'click',async()=>{const id=$('#adv-preset-select').value,p=S.presets.find(x=>x.id===id);if(!p)return;const bi=['default','web-h264','web-h265','web-vp9','archive-h264','audio-mp3','audio-aac','audio-opus','gif'];if(bi.includes(id)){diag('Cannot delete built-in preset');return}if(!await confirm('Delete preset "'+p.name+'"?'))return;try{await invoke('delete_preset',{id});S.presets=await invoke('get_presets');populatePresetDropdowns();diag('Preset deleted')}catch(e){diag('Delete error: '+e)}});
on($('#btn-save-preset'),'click',async()=>{const name=prompt('Preset name:');if(!name)return;const params=S.mode==='simple'?collectSimpleParams():collectAdvParams();const id=name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');const ex=S.presets.find(p=>p.id===id);if(ex){if(!await confirm(`Preset "${ex.name}" already exists. Overwrite?`))return}try{await invoke('create_preset',{preset:{id,name,description:'Custom preset',category:'Video',params}});S.presets=await invoke('get_presets');populatePresetDropdowns(id);diag('Preset saved: '+name)}catch(e){diag('Save error: '+e)}});
on($('#btn-delete-preset'),'click',async()=>{const id=$('#preset-select').value,p=S.presets.find(x=>x.id===id);if(!p)return;const bi=['default','web-h264','web-h265','web-vp9','archive-h264','audio-mp3','audio-aac','audio-opus','gif'];if(bi.includes(id)){diag('Cannot delete built-in preset');return}if(!await confirm(`Delete preset "${p.name}"?`))return;try{await invoke('delete_preset',{id});S.presets=await invoke('get_presets');populatePresetDropdowns();diag('Preset deleted')}catch(e){diag('Delete error: '+e)}});

// ── Populate preset dropdowns (with Custom option)
function populatePresetDropdowns(selId){
  const opts='<option value="">— Custom —</option>'+S.presets.map(p=>'<option value="'+p.id+'">'+p.name+'</option>').join('');
  ['#preset-select','#adv-preset-select'].forEach(sid=>{const sel=$(sid);if(sel){sel.innerHTML=opts;if(selId)sel.value=selId}})
}
on($('#btn-add-queue'),'click',()=>{if(!S.files.length){diag('No files');return}const bp=S.mode==='simple'?collectSimpleParams():collectAdvParams();const ext=bp.container.toLowerCase();const ow=[];S.files.forEach(f=>{const nm=f.split('/').pop(),st=nm.includes('.')?nm.substring(0,nm.lastIndexOf('.')):nm;let out=`${S.outputDir}/${st}.${ext}`;if(out===f){out=`${S.outputDir}/${st}_converted.${ext}`;ow.push(nm)}S.jobs.push({input:f,output:out,params:structuredClone(bp),status:'pending',progress:0})});S.files=[];renderFiles();renderQueue();if(ow.length){$('#status-text').textContent=`Renamed ${ow.length} to _converted`;$('#status-text').classList.add('text-[#f59e0b]');setTimeout(()=>$('#status-text').classList.remove('text-[#f59e0b]'),5000)}else{$('#status-text').textContent=`Queued ${S.jobs.length} job(s)`}});
function renderQueue(){const l=$('#queue-list');if(!S.jobs.length){l.innerHTML='';updateStatus();return}l.innerHTML=S.jobs.map(j=>{const inn=j.input.split('/').pop(),outn=j.output.split('/').pop();switch(j.status){case'pending':return`<div class="flex items-center justify-between text-xs py-1 px-2"><span class="text-[#c0c0d0] truncate">${inn} → ${outn}</span><span class="text-[#606070] shrink-0 ml-2">pending</span></div>`;case'running':{const w=Math.max(1,j.progress);return`<div id="job-running" class="text-xs py-1"><div class="flex justify-between text-[#c0c0d0] mb-1"><span class="truncate">${inn}</span><span class="progress-pct">${Math.round(j.progress)}%</span></div><div style="height:8px;background:#2a2a3a;border-radius:4px"><div class="progress-fill" style="height:8px;width:${w}%;background:#be4266;border-radius:4px"></div></div></div>`}case'done':return`<div class="flex items-center justify-between text-xs py-1 px-2"><span class="text-[#22c55e] truncate">✓ ${inn} → ${outn}</span><span class="text-[#22c55e] shrink-0 ml-2">done</span></div>`;case'failed':return`<div class="flex flex-col text-xs py-1 px-2"><div class="flex items-center justify-between"><span class="text-[#ef4444] truncate">✗ ${inn}</span><span class="text-[#ef4444] shrink-0 ml-2">failed</span></div><div class="text-[#606070] mt-1 text-[10px] cursor-pointer hover:text-[#c0c0d0]" title="${(j.error||'').replace(/"/g,'&quot;')}" onclick="navigator.clipboard.writeText(this.getAttribute('title'))">${(j.error||'unknown').substring(0,500)}</div></div>`;case'cancelled':return`<div class="flex items-center justify-between text-xs py-1 px-2"><span class="text-[#f59e0b] truncate">⊘ ${inn}</span><span class="text-[#f59e0b] shrink-0 ml-2">cancelled</span></div>`;default:return''}}).join('');updateStatus()}

// ── Re-queue + Clear ──
on($('#btn-requeue'),'click',()=>{const done=S.jobs.filter(j=>j.status==='done'||j.status==='failed'||j.status==='cancelled');if(!done.length){diag('No finished jobs');return}const files=done.map(j=>j.input);S.jobs=S.jobs.filter(j=>j.status==='pending'||j.status==='running');addFiles(files);renderQueue();diag('Re-run: '+files.length+' file(s)')});
on($('#btn-clear-queue'),'click',()=>{const before=S.jobs.length;S.jobs=S.jobs.filter(j=>j.status==='pending'||j.status==='running');renderQueue();diag('Cleared '+(before-S.jobs.length)+' finished')});

// ── Encoding ──
on($('#btn-start'),'click',async()=>{if(!S.jobs.length||S.isEncoding)return;const pend=S.jobs.filter(j=>j.status==='pending');if(!pend.length)return;const outs=pend.map(j=>j.output);try{const ex=await invoke('check_output_overwrite',{paths:outs});if(ex.length){if(!await confirm(`${ex.length} output file(s) already exist:\n${ex.map(p=>p.split('/').pop()).join(', ')}\n\nOverwrite?`)){diag('Cancelled: files exist');return}}}catch(e){}S.isEncoding=true;$('#btn-start').disabled=true;$('#btn-cancel').classList.remove('hidden');$('#status-text').textContent='Encoding...';const jd=pend.map(j=>({input_path:j.input,output_path:j.output,params:j.params}));try{await invoke('start_encoding',{jobData:jd,ffmpegPath:S.config?.ffmpeg_path||null})}catch(e){pend.forEach(j=>{j.status='failed';j.error=String(e)});renderQueue();finishEncoding();diag('ERROR: '+String(e).substring(0,100))}});
function finishEncoding(){S.isEncoding=false;$('#btn-start').disabled=false;$('#btn-cancel').classList.add('hidden')}
on($('#btn-cancel'),'click',async()=>{await invoke('cancel_encoding');S.jobs.forEach(j=>{if(j.status==='pending'||j.status==='running')j.status='cancelled'});renderQueue();finishEncoding();diag('Cancelled')});

// ── Events ──
async function setupEvents(){await listen('job:diag',e=>{const d=e.payload;$('#status-text').textContent=`Probe: ${d.file} ${d.probe_ok?'OK':'FAILED (no ffprobe)'}`});await listen('job:progress',e=>{const d=e.payload,j=S.jobs.find(x=>x.status==='running'||x.status==='pending');if(j){const wasPending=j.status==='pending';j.status='running';j.progress=d.progress_pct??0;if(wasPending||!$('#job-running')){renderQueue()}else{const w=Math.max(1,j.progress);const fill=$('#job-running .progress-fill');if(fill)fill.style.width=w+'%';const pct=$('#job-running .progress-pct');if(pct)pct.textContent=Math.round(j.progress)+'%'}diag('progress: '+Math.round(j.progress)+'%')}});await listen('job:done',e=>{const d=e.payload,j=S.jobs.find(x=>x.status==='running');if(j){j.status=d.status==='completed'?'done':(d.status==='cancelled'?'cancelled':'failed');j.error=d.error;j.command=d.command||''}renderQueue();if(S.jobs.every(x=>x.status==='done'||x.status==='failed'||x.status==='cancelled')){finishEncoding();loadHistory();const cmd=S.jobs.find(x=>x.command)?.command||'';$('#status-text').textContent='Complete'+(cmd?' — '+cmd.split(' ').slice(0,6).join(' ')+'...':'')}})}

// ── Modals ──
on($('#btn-settings'),'click',()=>$('#settings-modal').classList.remove('hidden'));on($('#btn-settings-cancel'),'click',()=>$('#settings-modal').classList.add('hidden'));on($('#btn-settings-save'),'click',async()=>{if(S.config)S.config.ffmpeg_path=$('#ffmpeg-path').value||null;await invoke('save_config',{config:S.config});$('#settings-modal').classList.add('hidden')});
on($('#btn-about'),'click',()=>$('#about-modal').classList.remove('hidden'));on($('#btn-about-close'),'click',()=>$('#about-modal').classList.add('hidden'));

// ── History ──
async function loadHistory(){try{S.history=await invoke('get_history');renderHistory()}catch(e){}}
function renderHistory(){const l=$('#history-list');if(!l)return;if(!S.history.length){l.innerHTML='<div class="text-[#606070] text-xs p-2">No jobs yet</div>';return}l.innerHTML=S.history.slice(-20).reverse().map(h=>{const inn=h.input_path.split('/').pop()||h.input_path,outn=h.output_path.split('/').pop()||h.output_path,date=new Date(h.created_at*1000).toLocaleString(),dur=h.duration_secs?`(${h.duration_secs.toFixed(1)}s)`:'',ok=h.status==='completed',icon=ok?'✓':h.status==='cancelled'?'⊘':'✗',color=ok?'text-[#22c55e]':h.status==='cancelled'?'text-[#f59e0b]':'text-[#ef4444]';return`<div class="flex items-center gap-2 text-xs py-1 px-2 rounded hover:bg-[#1f1f2e]"><span class="${color} w-4">${icon}</span><span class="text-[#c0c0d0] truncate flex-1" title="${h.input_path}">${inn} → ${outn}</span><span class="text-[#606070] shrink-0">${dur}</span></div>`}).join('')}
on($('#btn-clear-history'),'click',async()=>{await invoke('clear_history');S.history=[];renderHistory()});

// ── Init ──
async function init(){const d=[];d.push('TI:'+(!!window.__TAURI_INTERNALS__));try{S.presets=await invoke('get_presets');d.push('pre:'+S.presets.length);S.config=await invoke('get_config');S.outputDir=S.config?.output_dir||await invoke('get_default_output_dir');updateOutputDisplay();await setupEvents();await setupDragDrop();await loadHistory();d.push('READY')}catch(e){d.push('ERR:'+String(e).substring(0,60));S.outputDir='output';$('#output-dir').textContent=S.outputDir}diag(d.join(' | '));populatePresetDropdowns('default');updateProfileOptions();renderFilters();renderMetadata();renderFiles();updateCrfWarning();hookCmdPreview();updateCmdPreview()}
init();