/* The guide's behaviour. The page is whole without it (the contents, the
   chapters, every link and every example's three languages are written
   into the HTML); this adds the search, the theme switch, the examples'
   language switch, pictures shown larger, where you are, and the small
   comforts. */
(function () {
  "use strict";
  var doc = document;
  var root = doc.documentElement;
  var main = doc.getElementById("main");
  var toc = doc.getElementById("toc");
  var input = doc.getElementById("guide-search");
  var results = doc.getElementById("results");
  var progress = doc.getElementById("progress");
  var tocRead = doc.getElementById("toc-read");
  var crumbChapter = doc.getElementById("crumb-chapter");
  var crumbSection = doc.getElementById("crumb-section");
  var crumbSep = doc.querySelector("#crumb .sep");
  var toTop = doc.getElementById("to-top");
  var toast = doc.getElementById("toast");
  var each = function (list, f) { Array.prototype.forEach.call(list, f); };
  var store = {
    get: function (k) { try { return window.localStorage.getItem(k); } catch (e) { return null; } },
    set: function (k, v) { try { window.localStorage.setItem(k, v); } catch (e) { /* file:// or a private window */ } }
  };
  function here() { return location.pathname + location.search; }
  function setHash(id) {
    try { history.replaceState(null, "", id ? "#" + id : here()); } catch (e) { /* some file:// setups refuse */ }
  }
  var toastTimer = null;
  function say(text) {
    toast.textContent = text;
    toast.classList.add("shown");
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(function () { toast.classList.remove("shown"); }, 1600);
  }

  /* Theme: as the system (Auto), light or dark. The script in <head> has
     already set the stored one; this keeps the button right. */
  var themeButton = doc.getElementById("theme-button");
  var modes = ["system", "light", "dark"];
  var names = { system: "Auto", light: "Light", dark: "Dark" };
  var says = { system: "follows the system", light: "light", dark: "dark" };
  function applyTheme(t) {
    if (t === "light" || t === "dark") { root.setAttribute("data-theme", t); } else { root.removeAttribute("data-theme"); t = "system"; }
    themeButton.dataset.mode = t;
    themeButton.querySelector(".label").textContent = names[t];
    var next = modes[(modes.indexOf(t) + 1) % modes.length];
    themeButton.title = "Theme: " + names[t] + " (" + says[t] + "). Click for " + names[next] + ".";
    themeButton.setAttribute("aria-label", themeButton.title);
    return t;
  }
  var theme = applyTheme(store.get("guide-theme") || "system");
  themeButton.addEventListener("click", function () {
    theme = applyTheme(modes[(modes.indexOf(theme) + 1) % modes.length]);
    store.set("guide-theme", theme);
  });

  /* The examples' language: Lust, Rust or Lua, for every example at once,
     remembered like the theme. The script in <head> has already set the
     stored one. The example clicked stays where it is on the screen while
     the others above it change height. */
  var languages = ["lust", "rust", "lua"];
  function setCode(lang, keep) {
    if (languages.indexOf(lang) < 0) { lang = "lust"; }
    var before = keep ? keep.getBoundingClientRect().top : 0;
    root.setAttribute("data-code", lang);
    each(main.querySelectorAll(".example .langs button"), function (b) {
      b.setAttribute("aria-selected", b.dataset.lang === lang ? "true" : "false");
      b.tabIndex = b.dataset.lang === lang ? 0 : -1;
    });
    if (keep) { window.scrollBy(0, keep.getBoundingClientRect().top - before); }
  }
  setCode(root.getAttribute("data-code"), null);
  main.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest(".example .langs button");
    if (!b) { return; }
    setCode(b.dataset.lang, b);
    store.set("guide-code", b.dataset.lang);
  });
  main.addEventListener("keydown", function (e) {
    var b = e.target.closest && e.target.closest(".example .langs button");
    if (!b || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) { return; }
    e.preventDefault();
    var next = languages[(languages.indexOf(b.dataset.lang) + (e.key === "ArrowRight" ? 1 : 2)) % 3];
    setCode(next, b);
    store.set("guide-code", next);
    var to = b.parentNode.querySelector('[data-lang="' + next + '"]');
    if (to) { to.focus(); }
  });

  /* Pictures: clicking one shows it larger, over the page; a click, Esc or
     the close button puts it away. */
  var lightbox = doc.getElementById("lightbox");
  var lightboxPicture = doc.getElementById("lightbox-picture");
  var lightboxCaption = doc.getElementById("lightbox-caption");
  var lightboxFrom = null;
  function showLarger(button) {
    var img = button.querySelector("img");
    var caption = button.parentNode.querySelector("figcaption");
    lightboxPicture.src = img.src;
    lightboxPicture.alt = img.alt;
    lightboxCaption.textContent = caption ? caption.textContent : "";
    lightboxCaption.hidden = !caption;
    lightbox.hidden = false;
    doc.body.classList.add("boxed");
    lightboxFrom = button;
    doc.getElementById("lightbox-close").focus();
  }
  function putAway() {
    if (lightbox.hidden) { return false; }
    lightbox.hidden = true;
    doc.body.classList.remove("boxed");
    lightboxPicture.removeAttribute("src");
    if (lightboxFrom) { lightboxFrom.focus(); }
    return true;
  }
  main.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest("figure.shot .zoom");
    if (b) { showLarger(b); }
  });
  lightbox.addEventListener("click", putAway);

  function headingOf(el) {
    if (/^H[1-6]$/.test(el.tagName)) { return el; }
    return el.querySelector("h1, h2, h3, h4");
  }
  function titleOf(h) {
    var c = h.cloneNode(true);
    each(c.querySelectorAll(".anchor"), function (a) { a.remove(); });
    return c.textContent.replace(/\s+/g, " ").trim();
  }

  /* The chapters and their sections, in page order, from the contents
     written into the page. */
  var chapters = Array.prototype.slice.call(main.querySelectorAll("section.chapter"));
  var places = [];
  chapters.forEach(function (chapter) {
    var link = toc.querySelector('a[href="#' + chapter.id + '"]');
    if (!link) { return; }
    chapter.tocItem = link.parentNode;
    chapter.guideNum = chapter.dataset.num ? Number(chapter.dataset.num) : 0;
    chapter.guideTitle = chapter.id === "welcome" ? "Welcome" : titleOf(headingOf(chapter));
    places.push({ id: chapter.id, el: chapter, chapter: chapter, link: link, title: chapter.guideTitle, sub: false });
    each(chapter.querySelectorAll(":scope > section[id]"), function (sub) {
      var a = toc.querySelector('a[href="#' + sub.id + '"]');
      if (a) { places.push({ id: sub.id, el: sub, chapter: chapter, link: a, title: a.textContent, sub: true }); }
    });
  });

  /* Where you are: lit in the contents, named in the top bar, kept in the
     address (replaced, never added, so Back still leaves the page). */
  function topBar() { return doc.querySelector(".top").getBoundingClientRect().height; }
  var current = null;
  var hashTimer = null;
  var userScrolled = false;
  function readingLine() { return topBar() + 64; }
  function findCurrent() {
    var line = readingLine();
    var best = places[0];
    for (var i = 0; i < places.length; i++) {
      if (places[i].el.getBoundingClientRect().top <= line) { best = places[i]; } else { break; }
    }
    /* At the very bottom the last short sections never reach the line. */
    if (window.innerHeight + window.scrollY >= doc.documentElement.scrollHeight - 4) {
      for (var j = places.length - 1; j >= 0; j--) {
        if (places[j].el.getBoundingClientRect().top < window.innerHeight * 0.6) { best = places[j]; break; }
      }
    }
    return best;
  }
  function keepInView(a) {
    if (window.getComputedStyle(toc).visibility === "hidden") { return; }
    var tr = toc.getBoundingClientRect(), ar = a.getBoundingClientRect();
    if (ar.top < tr.top + 60 || ar.bottom > tr.bottom - 60) {
      toc.scrollTop += ar.top - tr.top - tr.height / 3;
    }
  }
  function show(place) {
    if (!place || place === current) { return; }
    var oldChapter = current && current.chapter;
    if (current && current.sub) { current.link.classList.remove("current"); current.link.removeAttribute("aria-current"); }
    if (oldChapter && oldChapter !== place.chapter) { oldChapter.tocItem.classList.remove("open", "active"); }
    current = place;
    var ch = place.chapter;
    ch.tocItem.classList.add("open", "active");
    if (place.sub) { place.link.classList.add("current"); place.link.setAttribute("aria-current", "location"); }
    crumbChapter.textContent = "";
    if (ch.guideNum > 0) {
      var n = doc.createElement("span");
      n.className = "num";
      n.textContent = ch.guideNum + ". ";
      crumbChapter.appendChild(n);
    }
    crumbChapter.appendChild(doc.createTextNode(ch.guideTitle));
    crumbChapter.href = "#" + ch.id;
    crumbSection.textContent = place.sub ? place.title : "";
    crumbSep.classList.toggle("hidden", !place.sub);
    keepInView(place.sub ? place.link : ch.tocItem.firstChild);
    if (userScrolled) {
      window.clearTimeout(hashTimer);
      hashTimer = window.setTimeout(function () {
        setHash(window.scrollY < 40 ? "" : current.id);
      }, 200);
    }
  }
  function onScroll() {
    var max = doc.documentElement.scrollHeight - window.innerHeight;
    var f = max > 0 ? Math.min(1, Math.max(0, window.scrollY / max)) : 0;
    progress.style.transform = "scaleX(" + f.toFixed(4) + ")";
    tocRead.textContent = Math.round(f * 100) + "% read";
    toTop.classList.toggle("shown", window.scrollY > 900);
    show(findCurrent());
  }
  var ticking = false;
  window.addEventListener("scroll", function () {
    if (!ticking) { ticking = true; window.requestAnimationFrame(function () { ticking = false; onScroll(); }); }
  }, { passive: true });
  ["wheel", "touchmove", "keydown"].forEach(function (t) {
    window.addEventListener(t, function () { userScrolled = true; }, { passive: true });
  });
  window.addEventListener("resize", onScroll);
  window.addEventListener("load", onScroll);
  onScroll();

  /* Landing somewhere lights its heading for a moment, so the eye finds it. */
  function landed(el) {
    var h = el && headingOf(el);
    if (!h) { return; }
    h.classList.remove("landed");
    void h.offsetWidth;
    h.classList.add("landed");
  }
  function goTo(id) {
    var el = doc.getElementById(id);
    if (!el) { return; }
    el.scrollIntoView({ block: "start" });
    landed(el);
  }
  doc.addEventListener("click", function (e) {
    var a = e.target.closest && e.target.closest('a[href^="#"]');
    if (!a || e.ctrlKey || e.metaKey || e.shiftKey) { return; }
    var id = decodeURIComponent(a.getAttribute("href").slice(1));
    if (!doc.getElementById(id)) { return; }
    e.preventDefault();
    if (a.classList.contains("anchor")) {
      setHash(id);
      var url = location.href;
      try {
        if (navigator.clipboard) { navigator.clipboard.writeText(url).then(function () { say("Link copied"); }, function () { say("Link in the address bar"); }); }
        else { say("Link in the address bar"); }
      } catch (err) { say("Link in the address bar"); }
      landed(doc.getElementById(id));
      return;
    }
    /* A link inside the page is a step you may want to go back from. */
    try { history.pushState(null, "", "#" + id); } catch (err) { /* refused */ }
    setToc(false);
    goTo(id);
    userScrolled = true;
    onScroll();
  });
  window.addEventListener("hashchange", function () { goTo(decodeURIComponent(location.hash.slice(1))); });
  window.addEventListener("popstate", function () {
    var id = decodeURIComponent(location.hash.slice(1));
    if (id) { goTo(id); } else { window.scrollTo(0, 0); }
  });
  if (location.hash.length > 1) {
    var start = doc.getElementById(decodeURIComponent(location.hash.slice(1)));
    if (start) { window.requestAnimationFrame(function () { start.scrollIntoView({ block: "start" }); landed(start); onScroll(); }); }
  }

  toTop.addEventListener("click", function () {
    window.scrollTo({ top: 0, behavior: "smooth" });
    setHash("");
    toc.scrollTop = 0;
  });

  /* The contents on a narrow screen: the button (or the line saying where
     you are) opens it; a link, the shade, or Esc closes it. */
  var tocButton = doc.getElementById("toc-button");
  function setToc(open) {
    doc.body.classList.toggle("toc-open", open);
    tocButton.setAttribute("aria-expanded", open ? "true" : "false");
    if (open && current) { keepInView(current.sub ? current.link : current.chapter.tocItem.firstChild); }
  }
  tocButton.addEventListener("click", function () { setToc(!doc.body.classList.contains("toc-open")); });
  doc.getElementById("scrim").addEventListener("click", function () { setToc(false); });
  doc.getElementById("crumb").addEventListener("click", function (e) {
    if (window.getComputedStyle(tocButton).display === "none") { return; }
    e.preventDefault();
    e.stopPropagation();
    setToc(true);
  }, true);

  /* Search. The guide is cut into places to find: each chapter's opening
     words and each of its sections. A place matches when every word typed
     is in it; the list says where each is, with the words around the
     match. The words are marked on the page while the search stands, and
     choosing a result goes to the first mark there and makes it flash. */
  var units = [];
  chapters.forEach(function (chapter) {
    if (!chapter.tocItem) { return; }
    var intro = [];
    each(chapter.children, function (n) {
      if (n.tagName === "SECTION" || n.classList.contains("pager")) { return; }
      intro.push(n);
    });
    units.push({ id: chapter.id, nodes: intro, chapter: chapter, title: chapter.guideTitle, intro: true });
    each(chapter.querySelectorAll(":scope > section[id]"), function (s) {
      var h = headingOf(s);
      units.push({ id: s.id, nodes: [s], chapter: chapter, title: h ? titleOf(h) : chapter.guideTitle });
    });
  });
  /* What a place says, without the examples' switches (which would
     match "Rust" on every page with an example). Every language's code is
     searched, shown or not. */
  function searchedText(n) {
    if (!n.querySelector || !n.querySelector(".langs")) { return n.textContent; }
    var c = n.cloneNode(true);
    each(c.querySelectorAll(".langs"), function (l) { l.remove(); });
    return c.textContent;
  }
  units.forEach(function (u) {
    u.text = u.nodes.map(searchedText).join(" ").replace(/\s+/g, " ").replace(/#(?=\s|$)/g, "").trim();
    u.lower = u.text.toLowerCase();
    u.body = u.text.indexOf(u.title) === 0 ? u.text.slice(u.title.length).trim() : u.text;
    u.bodyLower = u.body.toLowerCase();
  });

  function escapeHtml(s) { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;"); }
  function escapeRe(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

  function clearMarks() {
    var parents = [];
    each(main.querySelectorAll("mark.hl"), function (m) {
      var p = m.parentNode;
      p.replaceChild(doc.createTextNode(m.textContent), m);
      if (parents.indexOf(p) < 0) { parents.push(p); }
    });
    parents.forEach(function (p) { p.normalize(); });
  }
  function markWords(el, re) {
    var walker = doc.createTreeWalker(el, NodeFilter.SHOW_TEXT, {
      acceptNode: function (n) {
        var p = n.parentNode;
        if (!p || /^(SCRIPT|STYLE|MARK|BUTTON)$/.test(p.nodeName) || p.classList.contains("anchor")) { return NodeFilter.FILTER_REJECT; }
        re.lastIndex = 0;
        return re.test(n.nodeValue) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP;
      }
    });
    var nodes = [];
    while (walker.nextNode()) { nodes.push(walker.currentNode); }
    nodes.forEach(function (node) {
      var text = node.nodeValue, frag = doc.createDocumentFragment(), last = 0, m;
      re.lastIndex = 0;
      while ((m = re.exec(text)) !== null) {
        if (m[0].length === 0) { re.lastIndex++; continue; }
        frag.appendChild(doc.createTextNode(text.slice(last, m.index)));
        var mark = doc.createElement("mark");
        mark.className = "hl";
        mark.textContent = m[0];
        frag.appendChild(mark);
        last = m.index + m[0].length;
      }
      frag.appendChild(doc.createTextNode(text.slice(last)));
      node.parentNode.replaceChild(frag, node);
    });
  }
  function snippet(u, words) {
    var text = u.body, lower = u.bodyLower, at = -1;
    words.forEach(function (w) { var i = lower.indexOf(w); if (i >= 0 && (at < 0 || i < at)) { at = i; } });
    if (at < 0) { at = 0; }
    var from = Math.max(0, at - 60), to = Math.min(text.length, at + 160);
    if (from > 0) { var sp = text.indexOf(" ", from); if (sp >= 0 && sp < at) { from = sp + 1; } }
    var piece = (from > 0 ? "… " : "") + text.slice(from, to) + (to < text.length ? " …" : "");
    return escapeHtml(piece).replace(new RegExp("(" + words.map(function (w) { return escapeRe(escapeHtml(w)); }).join("|") + ")", "gi"), "<mark>$1</mark>");
  }

  var hits = [];
  var selected = 0;
  function setResults(open) {
    results.classList.toggle("shown", open);
    input.setAttribute("aria-expanded", open ? "true" : "false");
  }
  function select(i) {
    var items = results.querySelectorAll("a.result");
    if (!items.length) { return; }
    selected = (i + items.length) % items.length;
    each(items, function (a, k) { a.classList.toggle("selected", k === selected); a.setAttribute("aria-selected", k === selected ? "true" : "false"); });
    var a = items[selected], rr = results.getBoundingClientRect(), ar = a.getBoundingClientRect();
    if (ar.top < rr.top + 30) { results.scrollTop += ar.top - rr.top - 30; }
    else if (ar.bottom > rr.bottom - 40) { results.scrollTop += ar.bottom - rr.bottom + 40; }
  }
  function search(query) {
    clearMarks();
    each(toc.querySelectorAll("a"), function (a) { a.classList.remove("dim", "hit"); });
    each(toc.querySelectorAll("li.has-hits"), function (li) { li.classList.remove("has-hits"); });
    hits = [];
    var words = query.toLowerCase().split(/\s+/).filter(Boolean);
    if (!words.length) { results.innerHTML = ""; setResults(false); return; }
    var re = new RegExp(words.map(escapeRe).join("|"), "gi");
    var inTitle = [], inText = [];
    units.forEach(function (u) {
      if (!words.every(function (w) { return u.lower.indexOf(w) >= 0; })) { return; }
      var t = u.title.toLowerCase();
      (words.every(function (w) { return t.indexOf(w) >= 0; }) ? inTitle : inText).push(u);
    });
    hits = inTitle.concat(inText);
    var hitIds = {};
    hits.forEach(function (u) {
      u.nodes.forEach(function (n) { markWords(n, re); });
      hitIds[u.id] = true;
      u.chapter.tocItem.classList.add("has-hits");
    });
    places.forEach(function (p) {
      if (hitIds[p.id]) { p.link.classList.add("hit"); }
      else if (p.sub || !p.chapter.tocItem.classList.contains("has-hits")) { p.link.classList.add("dim"); }
    });
    var html = "";
    if (hits.length) {
      html += '<div class="count">' + hits.length + (hits.length === 1 ? " place matches" : " places match") + "</div>";
      hits.forEach(function (u, i) {
        var num = u.chapter.guideNum > 0 ? "Chapter " + u.chapter.guideNum : "Start";
        var where = u.intro ? num : num + " · " + u.chapter.guideTitle;
        html += '<a class="result" role="option" href="#' + escapeHtml(u.id) + '" data-i="' + i + '">' +
          '<span class="where">' + escapeHtml(where) + "</span>" +
          '<span class="title">' + escapeHtml(u.title) + "</span>" +
          '<span class="snippet">' + snippet(u, words) + "</span></a>";
      });
      html += '<div class="keys-help"><span><kbd>↑</kbd> <kbd>↓</kbd> choose</span><span><kbd>Enter</kbd> go there</span><span><kbd>Esc</kbd> clear</span></div>';
    } else {
      html = '<div class="empty">Nothing in the guide matches that. Try fewer or other words, such as “sprite”, “undo” or “physics”.</div>';
    }
    results.innerHTML = html;
    results.scrollTop = 0;
    setResults(true);
    select(0);
  }
  function jump(i) {
    var u = hits[i];
    if (!u) { return; }
    setResults(false);
    input.blur();
    /* The first match that shows; failing that, one in an example's other
       language, which the switch then shows. */
    var target = null, hidden = null;
    for (var k = 0; k < u.nodes.length && !target; k++) {
      if (!u.nodes[k].querySelectorAll) { continue; }
      each(u.nodes[k].querySelectorAll("mark.hl"), function (m) {
        if (target) { return; }
        if (m.getClientRects().length) { target = m; } else if (!hidden) { hidden = m; }
      });
    }
    var other = !target && hidden && hidden.closest("pre[data-lang]");
    if (other) { setCode(other.dataset.lang, null); target = hidden; }
    var el = target || u.nodes[0];
    /* The match a third of the way down, with its section's top above the
       reading line, so the contents and the top bar name that section. */
    var at = el.getBoundingClientRect().top + window.scrollY;
    var start = u.nodes[0].getBoundingClientRect().top + window.scrollY;
    var y = Math.max(at - window.innerHeight * 0.3, start - readingLine() + 1);
    window.scrollTo(0, Math.max(0, Math.min(y, at - topBar() - 24)));
    if (target) {
      each(main.querySelectorAll("mark.flash"), function (m) { m.classList.remove("flash"); });
      void target.offsetWidth;
      target.classList.add("flash");
    } else { landed(u.nodes[0]); }
    userScrolled = true;
    setHash(u.id);
    onScroll();
  }

  var timer = null;
  input.addEventListener("input", function () {
    window.clearTimeout(timer);
    timer = window.setTimeout(function () { search(input.value); }, 80);
  });
  function clearSearch() {
    window.clearTimeout(timer);
    input.value = "";
    search("");
  }
  input.addEventListener("keydown", function (e) {
    if (e.key === "ArrowDown") { e.preventDefault(); if (!results.classList.contains("shown")) { search(input.value); } else { select(selected + 1); } }
    else if (e.key === "ArrowUp") { e.preventDefault(); select(selected - 1); }
    else if (e.key === "Enter") {
      e.preventDefault();
      window.clearTimeout(timer);
      if (!results.classList.contains("shown") || !hits.length) { search(input.value); }
      jump(selected);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (results.classList.contains("shown") && input.value) { setResults(false); }
      else { clearSearch(); input.blur(); }
    }
  });
  input.addEventListener("focus", function () { if (input.value) { search(input.value); } });
  results.addEventListener("mousedown", function (e) { e.preventDefault(); });
  results.addEventListener("click", function (e) {
    var a = e.target.closest("a.result");
    if (!a) { return; }
    e.preventDefault();
    e.stopPropagation();
    jump(Number(a.dataset.i));
  }, true);
  results.addEventListener("mousemove", function (e) {
    var a = e.target.closest("a.result");
    if (a && Number(a.dataset.i) !== selected) { select(Number(a.dataset.i)); }
  });
  input.addEventListener("blur", function () { window.setTimeout(function () { setResults(false); }, 120); });

  doc.addEventListener("keydown", function (e) {
    var typing = /^(INPUT|TEXTAREA|SELECT)$/.test(doc.activeElement && doc.activeElement.tagName);
    if ((e.key === "k" || e.key === "K") && (e.ctrlKey || e.metaKey)) {
      e.preventDefault(); setToc(false); input.focus(); input.select();
    } else if (e.key === "/" && !typing) {
      e.preventDefault(); setToc(false); input.focus(); input.select();
    } else if (e.key === "Escape") {
      if (putAway()) { e.preventDefault(); }
      else if (doc.body.classList.contains("toc-open")) { setToc(false); }
      else if (!typing && input.value) { clearSearch(); }
    }
  });
}());
