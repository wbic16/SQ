// phext.ts
// this monstrosity of an implementation is basically Will's stream of consciousness
// if you want to refactor it for clarity, please submit a pr
// (c) 2023-2024 Phext, Inc.
// License: MIT

var upstream = 'https://github.com/wbic16/phextio';

var raw = "";
var phexts = {};
if (localStorage.raw && localStorage.raw.length > 0) {
  raw = localStorage.raw;
} else {
  raw = "<html>\n<head>\n<title>Reference Phext Document</title>\n</head>\n<body>\n\nYou are currently at 1.1.1/1.1.1/1.1.1.\nWe are aware of the following nodes:\n\n<a href='phext://1.1.1/1.1.1/1.1.2'>Scroll #2</a>\n<a href='phext://1.1.1/1.1.1/1.1.3'>Scroll #3</a>\n<a href='phext://1.1.1/1.1.1/1.1.4'>Scroll #4</a>\n\n</body>\n</html>Scroll #2\n---------\nThis happens to be a MarkDown file.\n\nScroll #3: Just a line of text.<html>\n<head>\n<title>Scroll #4</title>\n</head>\n<body>\n<a href='phext://1.1.1/1.1.1/1.1.1'>Return Home</a>\n\n<h1>Scroll #4</h1>\n</body>\n</html>";
  localStorage.raw = raw;
}

var phextMode = 'n'; // newbie

const LINE_BREAK       = '\n';   // 2nd (Newline)
const SCROLL_BREAK     = '\x17'; // 3rd
const SECTION_BREAK    = '\x18'; // 4th
const CHAPTER_BREAK    = '\x19'; // 5th
const BOOK_BREAK       = '\x1A'; // 6th
const VOLUME_BREAK     = '\x1C'; // 7th
const COLLECTION_BREAK = '\x1D'; // 8th
const SERIES_BREAK     = '\x1E'; // 9th
const SHELF_BREAK      = '\x1F'; // 10th
const LIBRARY_BREAK    = '\x01'; // 11th

const F1_KEY = 112;
const F2_KEY = 113;
const F3_KEY = 114;
const F4_KEY = 115;
const F5_KEY = 116;
const F6_KEY = 117;
const F7_KEY = 118;
const F8_KEY = 119;
const F9_KEY = 120;
const F10_KEY = 121;
const F11_KEY = 122;
const F12_KEY = 123;

var enterKey = LINE_BREAK;
var replacementDescription = "ENTER";
var priorButton = false;

var DimensionBreaks = new Array(
  SCROLL_BREAK,
  SECTION_BREAK,
  CHAPTER_BREAK,
  BOOK_BREAK,
  VOLUME_BREAK,
  COLLECTION_BREAK,
  SERIES_BREAK,
  SHELF_BREAK,
  LIBRARY_BREAK
);

// -----------------------------------------------------------------------------------------------------------
function defaultCoordinates() {
  var coordinates = new Array();
  coordinates[LINE_BREAK] = 1;
  coordinates[SCROLL_BREAK] = 1;
  coordinates[SECTION_BREAK] = 1;
  coordinates[CHAPTER_BREAK] = 1;
  coordinates[BOOK_BREAK] = 1;
  coordinates[VOLUME_BREAK] = 1;
  coordinates[COLLECTION_BREAK] = 1;
  coordinates[SERIES_BREAK] = 1;
  coordinates[SHELF_BREAK] = 1;
  coordinates[LIBRARY_BREAK] = 1;
  return coordinates;
}

var gl = false;
var ns = false;
var ta = false;
var cs = false;
var ls = false; // subspace
var te = false; // tiles
var tb = false; // tabs
var cx = false;
var cy = false;
var cz = false;
var wr = false;
var st = false; // subspace title
var cb = false; // command bar
var lk = false;
var cp = false;
var ha = false;
var sa = false;
var vr = false;
var qr = false;
var lt = false;
var sd = false;
var se = false;
var lts = false;
var qrui = false;
var qrd = false;
var qrl = false;
var qt = false;
var nodes = Array();
var qurl = "";
var mnt = false;
var mte = false;
var mss = false;
var tbe = false; // tab_editor
var tblb = false; // tab_library
var tbsf = false; // tab_shelf
var tbsr = false; // tab_series
var tbcn = false; // tab_collection
var tbvm = false; // tab_volume
var tbbk = false; // tab_book
var tbch = false; // tab_chapter
var tbsn = false; // tab_section
var tbsc = false; // tab_scroll

var tab_libraries = Array();
var tab_shelves = Array();
var tab_series = Array();
var tab_collections = Array();
var tab_volumes = Array();
var tab_books = Array();
var tab_chapters = Array();
var tab_sections = Array();
var tab_scrolls = Array();

var phextCoordinate = "1.1.1/1.1.1/1.1.1";

// -----------------------------------------------------------------------------------------------------------
function dgid(id) {
  return document.getElementById(id);
}

// -----------------------------------------------------------------------------------------------------------
function loadVars() {
  if (!gl) { gl = dgid("goal"); }
  if (!ns) { ns = dgid("nodes"); }
  if (!ta) { ta = dgid("scroll"); }
  if (!cs) { cs = dgid("coords"); }
  if (!ls) { ls = dgid("subspace"); }
  if (!te) { te = dgid("tiles"); }
  if (!tb) { tb = dgid("tabs"); }
  if (!cx) { cx = dgid("coordsX"); }
  if (!cy) { cy = dgid("coordsY"); }
  if (!cz) { cz = dgid("coordsZ"); }
  if (!wr) { wr = dgid("whiterabbit"); }
  if (!st) { st = dgid("subspaceTitle"); }
  if (!cb) { cb = dgid("commandBar"); }
  if (!cp) { cp = dgid("coordinatePlate"); }
  if (!ha) { ha = dgid("helparea"); }
  if (!sa) { sa = dgid("subspaceArea"); }
  if (!lt) { lt = dgid("linkerText"); }
  if (!sd) { sd = dgid("seed"); }
  if (!se) { se = dgid("seeds"); }
  if (!lts) { lts = dgid("linkerStatus"); }
  if (!qrui) { qrui = dgid("qrcode"); }
  if (!qrl) { qrl = dgid("qrlabel"); }
  if (!qt) { qt = dgid("quest"); }
  if (!mnt) { mnt = dgid("modeNestedTabs"); }
  if (!mte) { mte = dgid("modeTiles"); }
  if (!mss) { mss = dgid("modeSubspace"); }
  if (!te) { tbe = dgid("tab_editor"); }
  if (!tblb) { tblb = dgid("tab_library"); }
  if (!tbsf) { tbsf = dgid("tab_shelf"); }
  if (!tbsr) { tbsr = dgid("tab_series"); }
  if (!tbcn) { tbcn = dgid("tab_collection"); }
  if (!tbvm) { tbvm = dgid("tab_volume"); }
  if (!tbbk) { tbbk = dgid("tab_book"); }
  if (!tbch) { tbch = dgid("tab_chapter"); }
  if (!tbsn) { tbsn = dgid("tab_section"); }
  if (!tbsc) { tbsc = dgid("tab_scroll"); }
  for (var i = 1; i <= 7; ++i)
  {
    tab_libraries[i] = dgid("lb" + i);
    tab_shelves[i] = dgid("sf" + i);
    tab_series[i] = dgid("sr" + i);
    tab_collections[i] = dgid("cn" + i);
    tab_volumes[i] = dgid("vm" + i);
    tab_books[i] = dgid("bk" + i);
    tab_chapters[i] = dgid("ch" + i);
    tab_sections[i] = dgid("sn" + i);
    tab_scrolls[i] = dgid("sc" + i);
  }

  if (localStorage.seed) {
    sd.value = localStorage.seed;
  }
}

// -----------------------------------------------------------------------------------------------------------
function updateCoordinate() {
  phextCoordinate = cx.value + "/" + cy.value + "/" + cz.value;
}

var coords = defaultCoordinates();
var target = defaultCoordinates();
var scrollFound = false;

// -----------------------------------------------------------------------------------------------------------
function loadPhext() {
  raw = ls.value;
  coords = defaultCoordinates();
  var gz = cz.value.split('.');
  var gy = cy.value.split('.');
  var gx = cx.value.split('.');
  if (gz.length >= 1) { target[LIBRARY_BREAK] = gz[0]; }
  if (gz.length >= 2) { target[SHELF_BREAK] = gz[1]; }
  if (gz.length >= 3) { target[SERIES_BREAK] = gz[2]; }
  if (gy.length >= 1) { target[COLLECTION_BREAK] = gy[0]; }
  if (gy.length >= 2) { target[VOLUME_BREAK] = gy[1]; }
  if (gy.length >= 3) { target[BOOK_BREAK] = gy[2]; }
  if (gx.length >= 1) { target[CHAPTER_BREAK] = gx[0]; }
  if (gx.length >= 2) { target[SECTION_BREAK] = gx[1]; }
  if (gx.length >= 3) { target[SCROLL_BREAK] = gx[2]; }

  scrollFound = false;
  qt.innerHTML = "";
  nodes = Array();
  totalScrolls = 1;
  var libs = raw.split(LIBRARY_BREAK);  
  libs.forEach((library) => processLibrary(library));
  if (!scrollFound) {
    ta.value = "";
  }
  qt.innerHTML = nodes.join("\n");
}

// -----------------------------------------------------------------------------------------------------------
// @fn dimensionBreak
// -----------------------------------------------------------------------------------------------------------
function dimensionBreak(type) {
  if (type == LIBRARY_BREAK) {
    ++coords[LIBRARY_BREAK];
    coords[SHELF_BREAK] = 1;
    coords[SERIES_BREAK] = 1;
    coords[COLLECTION_BREAK] = 1;
    coords[VOLUME_BREAK] = 1;
    coords[BOOK_BREAK] = 1;
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == SHELF_BREAK) {
    ++coords[SHELF_BREAK];
    coords[SERIES_BREAK] = 1;
    coords[COLLECTION_BREAK] = 1;
    coords[VOLUME_BREAK] = 1;
    coords[BOOK_BREAK] = 1;
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == SERIES_BREAK) {
    ++coords[SERIES_BREAK];
    coords[COLLECTION_BREAK] = 1;
    coords[VOLUME_BREAK] = 1;
    coords[BOOK_BREAK] = 1;
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == COLLECTION_BREAK) {
    ++coords[COLLECTION_BREAK];
    coords[VOLUME_BREAK] = 1;
    coords[BOOK_BREAK] = 1;
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == VOLUME_BREAK) {
    ++coords[VOLUME_BREAK];
    coords[BOOK_BREAK] = 1;
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == BOOK_BREAK) {
    ++coords[BOOK_BREAK];
    coords[CHAPTER_BREAK] = 1;
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == CHAPTER_BREAK) {
    ++coords[CHAPTER_BREAK];
    coords[SECTION_BREAK] = 1;
    coords[SCROLL_BREAK] = 1;
  }

  if (type == SECTION_BREAK) {
    ++coords[SECTION_BREAK];
    coords[SCROLL_BREAK] = 1;
  }

  if (type == SCROLL_BREAK) {
    ++coords[SCROLL_BREAK];
  }
}

// -----------------------------------------------------------------------------------------------------------
function processLibrary(library) {
  var shelves = library.split(SHELF_BREAK);
  shelves.forEach((shelf) => processShelf(shelf));
  dimensionBreak(LIBRARY_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processShelf(shelf) {
  var series = shelf.split(SERIES_BREAK);
  series.forEach((seri) => processSeries(seri));
  dimensionBreak(SHELF_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processSeries(seri) {
  var collections = seri.split(COLLECTION_BREAK);
  collections.forEach((collection) => processCollection(collection));
  dimensionBreak(SERIES_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processCollection(collection) {
  var volumes = collection.split(VOLUME_BREAK);
  volumes.forEach((volume) => processVolume(volume));
  dimensionBreak(COLLECTION_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processVolume(volume) {
  var books = volume.split(BOOK_BREAK);
  books.forEach((book) => processBook(book));
  dimensionBreak(VOLUME_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processBook(book) {
  var chapters = book.split(CHAPTER_BREAK);
  chapters.forEach((chapter) => processChapter(chapter));
  dimensionBreak(BOOK_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processChapter(chapter) {
  var sections = chapter.split(SECTION_BREAK);
  sections.forEach((section) => processSection(section));
  dimensionBreak(CHAPTER_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function processSection(section) {
  var sections = section.split(SCROLL_BREAK);
  sections.forEach((scroll) => processScroll(scroll));
  dimensionBreak(SECTION_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function coordinatesMatch(a, b) {
  return a[SCROLL_BREAK]     == b[SCROLL_BREAK] &&
         a[SECTION_BREAK]    == b[SECTION_BREAK] &&
         a[CHAPTER_BREAK]    == b[CHAPTER_BREAK] &&
         a[BOOK_BREAK]       == b[BOOK_BREAK] &&
         a[VOLUME_BREAK]     == b[VOLUME_BREAK] &&
         a[COLLECTION_BREAK] == b[COLLECTION_BREAK] &&
         a[SERIES_BREAK]     == b[SERIES_BREAK] &&
         a[SHELF_BREAK]      == b[SHELF_BREAK] &&
         a[LIBRARY_BREAK]    == b[LIBRARY_BREAK];
}

// -----------------------------------------------------------------------------------------------------------
function coordToString(coords) {
  var chz = coords[LIBRARY_BREAK] + "." + coords[SHELF_BREAK] + "." + coords[SERIES_BREAK];
  var chy = coords[COLLECTION_BREAK] + "." + coords[VOLUME_BREAK] + "." + coords[BOOK_BREAK];
  var chx = coords[CHAPTER_BREAK] + "." + coords[SECTION_BREAK] + "." + coords[SCROLL_BREAK];

  return chz + "/" + chy + "/" + chx;
}

// -----------------------------------------------------------------------------------------------------------
function coordinateHit(coords) {

  var chz = coords[LIBRARY_BREAK] + "." + coords[SHELF_BREAK] + "." + coords[SERIES_BREAK];
  var chy = coords[COLLECTION_BREAK] + "." + coords[VOLUME_BREAK] + "." + coords[BOOK_BREAK];
  var chx = coords[CHAPTER_BREAK] + "." + coords[SECTION_BREAK] + "." + coords[SCROLL_BREAK];

  return "<a href='" + getPhextUrl(chx, chy, chz) + "'>@" + chz + "/" + chy + "/" + chx + "</a>";
}

// -----------------------------------------------------------------------------------------------------------
function drawNode(coords, scroll) {
  var scrollID = coords[SCROLL_BREAK];

  var text = scroll;
  const sizeLimit = 250;
  if (scroll.length > sizeLimit)
  {
    text = scroll.substring(0, sizeLimit);
    text += "...";
  }
  
  var node = "<div class='node' onclick='loadNode(" + coordToString(coords) + ");'>" + coordinateHit(coords) + " <input type='button' onclick='editScroll(" + coords + ");' value='Edit' /><br />" + text + "</div>";

  return node;
}

// -----------------------------------------------------------------------------------------------------------
// @fn processScroll
// -----------------------------------------------------------------------------------------------------------
var totalScrolls = 1;
function processScroll(scroll) {  
  if (coordinatesMatch(target, coords)) {
    ta.value = scroll;
    scrollFound = true;
    var lineCount = scroll.split('\n').length;
    if (lineCount < 100) { ta.rows = lineCount; }
    else { ta.rows = 100; }
  }
  if (scroll.length > 0) {
    // todo: let's keep nodes as a sparse tree of scrolls with content
    // that way, we can render tiles in a hierarchy as well, and then zooming makes sense
    nodes[totalScrolls] = drawNode(coords, scroll);
    ++totalScrolls;
  }
  dimensionBreak(SCROLL_BREAK);
}

// -----------------------------------------------------------------------------------------------------------
function safeEncode(text) {
  if (!text || text.length < 1) { return ""; }
  return encodeURIComponent(text.replaceAll("'", "%27"));
}

function updateQR() {
  if (!qr) {
    qr = new QRCode(document.getElementById("qrcode"), {text: "https://phext.io/index.html", width: 640, height: 640});
    qrui.style.background = '#000';
  }  
  qr.clear();
  qr.makeCode(qurl);
}

// -----------------------------------------------------------------------------------------------------------
var timeoutDelay = 1000;
function getPhextUrl(x, y, z) {
  qurl = "https://phext.io/index.html?seed=" + safeEncode(sd.value) + "&cz=" + safeEncode(z) + "&cy=" + safeEncode(y) + "&cx=" + safeEncode(x) + "&" + phextMode + "=" + safeEncode(raw);
  lt.value = qurl;
  if (qrd) {
    clearTimeout(qrd);
  }
  qrd = setTimeout(updateQR, timeoutDelay);
  lts.innerHTML = "";
  return qurl;
}

// @fn greypill
function greypill() {
  gl.style.display = 'block';
  ns.style.display = 'block';
  wr.style.display = 'none';
  ta.style.display = 'none';
  cp.style.display = 'none';
  ha.style.display = 'none';
  sa.style.display = 'none';
  qrui.style.display = 'block';
  qrl.style.display = 'none';
  qt.style.display = 'none';
}

// -----------------------------------------------------------------------------------------------------------
// @fn redpill
// -----------------------------------------------------------------------------------------------------------
function redpill(store) {
  phextMode = 'r';
  st.innerHTML = "Follow the <a class='small' href='white-rabbit.html'>White Rabbit</a>.";
  gl.style.display = 'none';
  ns.style.display = 'none';
  wr.style.display = 'block';
  ta.style.display = 'block';
  cp.style.display = 'block';
  ha.style.display = 'block';
  sa.style.display = 'block';
  qrui.style.display = 'block';
  qrl.style.display = 'block';
  qt.style.display = 'block';

  updateCoordinate();
  loadPhext();
  var ignored = getPhextUrl(cx.value, cy.value, cz.value) + "#RedPill";

  if (store) {
    saveContent();
  }
}

// -----------------------------------------------------------------------------------------------------------
// @fn saveContent
// -----------------------------------------------------------------------------------------------------------
function saveContent() {
  localStorage.raw = ls.value;
  if (!sd.value || sd.value.trim().length == 0) {
    sd.value = "holiday";
  }
  if (!phexts[sd.value]) {
    var opt = document.createElement('option');
    opt.value = sd.value;
    opt.innerHTML = sd.value;
    opt.selected = true;
    se.appendChild(opt);
  }
  phexts[sd.value] = ls.value;
  localStorage.seed = sd.value;
  localStorage.phexts = JSON.stringify(phexts);
}

// -----------------------------------------------------------------------------------------------------------
// @fn bluepill
// -----------------------------------------------------------------------------------------------------------
function bluepill() {
  phextMode = 'b';
  gl.style.display = 'none';
  ns.style.display = 'none';
  wr.style.display = 'none';
  ta.style.display = 'none';
  cp.style.display = 'none';
  ha.style.display = 'none';
  sa.style.display = 'block';
  qrui.style.display = 'none';
  qrl.style.display = 'none';
  qt.style.display = 'none';

  ta.value = raw;
  ta.rows = raw.split('\n').length;

  st.innerHTML = "Believe";
  ta.value = raw;

  dgid("linker").href = getPhextUrl(cx.value, cy.value, cz.value) + "#BluePill";
}

// -----------------------------------------------------------------------------------------------------------
// @fn whitepill
// -----------------------------------------------------------------------------------------------------------
function whitepill() {
  window.open(upstream);
}

// -----------------------------------------------------------------------------------------------------------
// @fn whiteRabbit
// -----------------------------------------------------------------------------------------------------------
function whiteRabbit() {
  window.open('whiterabbit.html');
}

// -----------------------------------------------------------------------------------------------------------
// @fn copyUrl
// -----------------------------------------------------------------------------------------------------------
function copyUrl() {
  getPhextUrl();
  saveContent();
  navigator.clipboard.writeText(qurl);
  lts.innerHTML = "URL copied!";
}

function setValue(id, text) {
  var handle = dgid(id);
  if (handle && handle.value) {
    handle.value = text;
  }
}

// -----------------------------------------------------------------------------------------------------------
// @fn startup
// -----------------------------------------------------------------------------------------------------------
function startup() {
  setValue("pscrollbreak", SCROLL_BREAK);
  setValue("psectionbreak", SECTION_BREAK);
  setValue("pchapterbreak", CHAPTER_BREAK);
  setValue("pbookbreak", BOOK_BREAK);
  setValue("pvolumebreak", VOLUME_BREAK);
  setValue("pcollectionbreak", COLLECTION_BREAK);
  setValue("pseriesbreak", SERIES_BREAK);
  setValue("pshelfbreak", SHELF_BREAK);
  setValue("plibrarybreak", LIBRARY_BREAK);
  loadVars();
  edit('tabs');  

  var urlSearchParams = new URLSearchParams(window.location.search);
  var params = Object.fromEntries(urlSearchParams.entries());

  if (params.cz) {
    cz.value = params.cz;
  }
  if (params.cy) {
    cy.value = params.cy;
  }
  if (params.cx) {
    cx.value = params.cx;
  }

  if (params.seed) {
    localStorage.seed = params.seed;
    sd.value = params.seed;
  }

  if (localStorage.phexts) {
    phexts = JSON.parse(localStorage.phexts);
    Object.keys(phexts).forEach((key) => {
      var opt = document.createElement('option');
      opt.value = key;
      opt.innerHTML = key;
      if (key == sd.value) { opt.selected = true; }
      se.appendChild(opt);
    });
  }

  if (phexts && phexts[params.seed]) {
    localStorage.raw = localStorage.phexts[params.seed];
  }
  if (localStorage.raw) {
    raw = localStorage.raw;
  }

  if (ls) {
    ls.value = raw;
  }

  if (params.r) {
    raw = params.r.replaceAll("%27", "'");
    ls.value = raw;
    redpill(false);
  }
  if (params.b) {
    raw = params.b.replaceAll("%27", "'");
    ls.value = raw;
    bluepill();
  }
}

// -----------------------------------------------------------------------------------------------------------
// @fn setEnterType
// -----------------------------------------------------------------------------------------------------------
function setEnterType(number, dimension, replacement) {
  enterKey = replacement;
  var handle = dgid("CB" + number + "_" + dimension);
  if (!handle) { return; }
  if (priorButton) {
    priorButton.style.borderStyle = '';
    priorButton.style.backgroundColor = '';
  }
  handle.style.borderStyle = 'inset';
  handle.style.backgroundColor = 'orange';
  replacementDescription = '&lt;' + handle.value + '&gt;';
  priorButton = handle;
}

// -----------------------------------------------------------------------------------------------------------
// @fn phextMods
// -----------------------------------------------------------------------------------------------------------
function phextMods(editor, e) {
  if (!remapFunctionKeys[editor]) {
    return;
  }
  for (var i = F1_KEY; i <= F10_KEY; ++i)
  {
    if (e.keyCode == i) {
      e.preventDefault();
      var button = dgid("CB" + editor + "_" + (e.keyCode - F1_KEY + 2));
      if (button) {
        button.click();
      }
      continue;
    }
  }
  if (!(e.key === "Enter")) {
    return;
  }
  e.preventDefault();
  var console = ls;
  var el = document.activeElement;

  const start = el.selectionStart;
  const before = el.value.substring(0, start);
  const after  = el.value.substring(el.selectionEnd, el.value.length);

  const sequence = (enterKey == LINE_BREAK) ? enterKey : enterKey + LINE_BREAK;
  el.value = (before + sequence + after);
  el.selectionStart = el.selectionEnd = start + 1;
  el.focus();
}

var remapFunctionKeys = new Array();
remapFunctionKeys[1] = true;
remapFunctionKeys[2] = true;

// -----------------------------------------------------------------------------------------------------------
// @fn toggle
// -----------------------------------------------------------------------------------------------------------
function toggle(editor) {
  var toggleButton = dgid("disabler" + editor);
  if (!toggleButton) { return; }
  const enabled = toggleButton.value === "Disable";
  toggleButton.value = enabled ? "Enable" : "Disable";
  remapFunctionKeys[editor] = enabled;
}

// -----------------------------------------------------------------------------------------------------------
// @fn chooseSeed
// -----------------------------------------------------------------------------------------------------------
function chooseSeed(seed) {
  sd.value = seed.value;
  ls.value = phexts[seed.value];
}

// -----------------------------------------------------------------------------------------------------------
// @fn removeSeed
// -----------------------------------------------------------------------------------------------------------
function removeSeed() {
  if (!phexts[sd.value]) {
    return;
  }
  delete phexts[sd.value];
  Object.keys(se.options).forEach((key) => {
    if (se.options[key] && se.options[key].value == sd.value) {
      se.options.remove(key);
    }
  });
  sd.value = se.options[se.selectedIndex].value;
  localStorage.seed = sd.value;
  localStorage.phexts = JSON.stringify(phexts);
}

// -----------------------------------------------------------------------------------------------------------
// @fn edit
// -----------------------------------------------------------------------------------------------------------
function edit(mode) {
  if (sd) { sd.style.display = 'block'; }
  if (cp) { cp.style.display = 'block'; }

  var tab_mode = mode == 'tabs';
  var tab_item = tab_mode ? 'block' : 'none';
  if (tb) { tb.style.display = tab_item; }
  if (mnt) { mnt.style.border = tab_mode ? "2px solid" : ""; }

  var tile_mode = mode == 'tiles';
  var tile_item = tile_mode ? 'block' : 'none';
  if (te) { te.style.display = tile_item; }
  if (mte) {
    mte.style.border = tile_mode ? "2px solid" : "";
  }

  var subspace_mode = mode == 'subspace';
  var subspace_item = subspace_mode ? 'block' : 'none';
  if (mss) { mss.style.border = subspace_mode ? "2px solid" : ""; }
  if (ls) { ls.style.display = subspace_item; }
  if (st) { st.style.display = subspace_item; }
  if (ta) { ta.style.display = subspace_item; }
  if (cb) { cb.style.display = subspace_item; }

  if (tbsf) { tbsf.style.display = 'none'; }
  if (tbsr) { tbsr.style.display = 'none'; }
  if (tbcn) { tbcn.style.display = 'none'; } 
  if (tbvm) { tbvm.style.display = 'none'; }
  if (tbbk) { tbbk.style.display = 'none'; }
  if (tbch) { tbch.style.display = 'none'; }
  if (tbsn) { tbsn.style.display = 'none'; }
  if (tbsc) { tbsc.style.display = 'none'; }
}

function tab_break(level) {
  tbsf.style.display = level >= 1 ? 'block' : 'none';
  tbsr.style.display = level >= 2 ? 'block' : 'none';
  tbcn.style.display = level >= 3 ? 'block' : 'none';
  tbvm.style.display = level >= 4 ? 'block' : 'none';
  tbbk.style.display = level >= 5 ? 'block' : 'none';
  tbch.style.display = level >= 6 ? 'block' : 'none';
  tbsn.style.display = level >= 7 ? 'block' : 'none';
  tbsc.style.display = level >= 8 ? 'block' : 'none';
}

function jump(sender, type, coordinate) {
  var czs = cz.value.split('.');
  var cys = cy.value.split('.');
  var cxs = cx.value.split('.');  
  
  // pick library coordinate
  if (type == 'lb') {    
    tab_break(1);
    czs[0] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_libraries[i].style.border = "";
      tab_libraries[i].style.backgroundColor = "";
    }
  }

  // pick shelf coordinate
  if (type == 'sf') {
    tab_break(2);
    czs[1] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_shelves[i].style.border = "";
      tab_shelves[i].style.backgroundColor = "";
    }
  }

  // pick series coordinate
  if (type == 'sr') {
    tab_break(3);
    czs[2] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_series[i].style.border = "";
      tab_series[i].style.backgroundColor = "";
    }
  }

  // pick collection coordinate
  if (type == 'cn') {
    tab_break(4);
    cys[0] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_collections[i].style.border = "";
      tab_collections[i].style.backgroundColor = "";
    }
  }

  // pick volume coordinate
  if (type == 'vm') {
    tab_break(5);
    cys[1] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_volumes[i].style.border = "";
      tab_volumes[i].style.backgroundColor = "";
    }
  }

  // pick book coordinate
  if (type == 'bk') {
    tab_break(6);
    cys[2] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_books[i].style.border = "";
      tab_books[i].style.backgroundColor = "";
    }
  }

  // pick chapter coordinate
  if (type == 'ch') {
    tab_break(7);
    cxs[0] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_chapters[i].style.border = "";
      tab_chapters[i].style.backgroundColor = "";
    }
  }

  // pick section coordinate
  if (type == 'sn') {
    tab_break(8);
    cxs[1] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_sections[i].style.border = "";
      tab_sections[i].style.backgroundColor = "";
    }
  }

  // pick scroll coordinate
  if (type == 'sc') {
    tab_break(9);
    cxs[2] = coordinate;
    for (var i = 1; i <= 7; ++i)
    {
      tab_scrolls[i].style.border = "";
      tab_scrolls[i].style.backgroundColor = "";
    }
  }

  sender.style.border = "1px solid orange";
  sender.style.backgroundColor = "#6560e0";
  cz.value = czs[0] + "." + czs[1] + "." + czs[2];
  cy.value = cys[0] + "." + cys[1] + "." + cys[2];
  cx.value = cxs[0] + "." + cxs[1] + "." + cxs[2];
}

function tab_value(indicator, index) {
  return parseInt(tab_libraries[index].value.replace(indicator, "")) - 1;
}

function tab_shift(type, op) {
  var indicator = "";
  if (type == 'lb') { indicator = "Library "; }
  if (type == 'sf') { indicator = "Shelf "; }
  if (type == 'sr') { indicator = "Series "; }
  if (type == 'cn') { indicator = "Collection "; }
  if (type == 'vm') { indicator = "Volume "; }
  if (type == 'bk') { indicator = "Book "; }
  if (type == 'ch') { indicator = "Chapter "; }
  if (type == 'sn') { indicator = "Section "; }
  if (type == 'sc') { indicator = "Scroll "; }
  if (indicator.length == 0) { return; }
  var start = tab_value(indicator, 1);
  if (op == 'add') { start = start + 1; }
  if (op == 'sub') { start = start - 1; }
  if (start < 0) { start = 0; }
  if (start > 100) { start = 100; }
  
  for (var i = 1; i <= 7; ++i) {    
    tab_libraries[i].value = indicator + (i + start);
  }
}