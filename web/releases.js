/* Render danh sach ban phat hanh tu /api/releases (tach rieng vi CSP chan <script> inline). */
(function(){
  var box=document.getElementById('vf-rel-list'); if(!box) return;
  var api=box.getAttribute('data-api')||'/api/releases';
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function md(s){
    var t=esc(s);
    t=t.replace(/^\s*[-*\u2022]\s+/,'');
    t=t.replace(/`([^`]+)`/g,'<code>$1</code>');
    t=t.replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>');
    t=t.replace(/(^|[^*])\*([^*\s][^*]*)\*/g,'$1<em>$2</em>');
    t=t.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,'<a href="$2" target="_blank" rel="noopener">$1</a>');
    return t;
  }
  function fmt(d){try{return new Date(d).toLocaleDateString('vi-VN',{year:'numeric',month:'2-digit',day:'2-digit'});}catch(e){return '';}}
  fetch(api,{headers:{'Accept':'application/json'}}).then(function(r){return r.json();}).then(function(d){
    var items=(d&&d.releases)||[];
    if(!items.length){box.innerHTML='<p class="vf-rel-empty">Chưa có bản phát hành chính thức — theo dõi repo để nhận bản đầu tiên.</p>';return;}
    box.innerHTML=items.map(function(r){
      return '<article class="vf-rel-card">'
        +'<span class="vf-rel-tag">'+esc(r.tag)+(r.prerelease?' · pre':'')+'</span>'
        +'<p class="vf-rel-name">'+esc(r.name||r.tag)+'</p>'
        +'<p class="vf-rel-meta">'+esc(fmt(r.published_at))+'</p>'
        +(r.summary?'<p class="vf-rel-sum">'+md(r.summary)+'</p>':'')
        +'<a href="'+esc(r.url)+'" target="_blank" rel="noopener">Xem bản phát hành →</a>'
        +'</article>';
    }).join('');
  }).catch(function(){box.innerHTML='<p class="vf-rel-empty">Không tải được danh sách bản phát hành.</p>';});
})();
