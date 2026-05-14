// Kleis Graph Editor — Domain-Agnostic
//
// Graph editor for building graph() EditorNode ASTs.
// Component definitions are loaded from /api/templates at startup.
// Any .kleist template with "ports" metadata becomes a graph-editable component.
//
// Data model:
//   - Components: typed entities with ports placed at (x,y) on canvas
//   - Nets: sets of connected ports (rows in the incidence matrix)
//   - Incidence matrix: signed sparse COO (nets x ports)

// ---------------------------------------------------------------------------
// Component definitions — loaded from server at init
// ---------------------------------------------------------------------------

let COMPONENT_DEFS = {};
let PALETTE_SECTIONS = [];

const DOMAIN_FILTER = new URLSearchParams(window.location.search).get('domain');

const domainConfig = {
    routing_mode: 'orthogonal',
    junction_style: 'dot',
    multi_port_strategy: 'trunk_branch',
    edge_decoration: 'none',
    edge_direction: 'undirected',
};

function parsePorts(portStr) {
    const ports = {};
    for (const segment of portStr.split(';')) {
        const [name, coords] = segment.split(':');
        if (!name || !coords) continue;
        const [x, y] = coords.split(',').map(Number);
        ports[name.trim()] = [x, y];
    }
    return ports;
}

function parseParams(paramStr) {
    if (!paramStr) return [];
    const params = [];
    for (const segment of paramStr.split(';')) {
        const parts = segment.split(':');
        if (parts.length < 3) continue;
        const [name, type, defaultVal] = parts.map(s => s.trim());
        params.push({
            name,
            type,
            default: type === 'int' ? parseInt(defaultVal, 10) : parseFloat(defaultVal),
        });
    }
    return params;
}

function categoryLabel(cat) {
    const suffix = cat.split('_').slice(1).join(' ');
    return suffix.charAt(0).toUpperCase() + suffix.slice(1);
}

async function loadComponentDefs() {
    const res = await fetch('/api/templates');
    const templates = await res.json();

    COMPONENT_DEFS = {};
    const sectionMap = {};

    for (const t of templates) {
        const meta = t.metadata || {};

        if (t.name.startsWith('__domain_')) {
            if (DOMAIN_FILTER && !t.name.includes(DOMAIN_FILTER)) continue;
            for (const key of Object.keys(domainConfig)) {
                if (meta[key]) domainConfig[key] = meta[key];
            }
            for (const key of Object.keys(meta)) {
                if (key.startsWith('verify_')) domainConfig[key] = meta[key];
            }
            continue;
        }

        if (!meta.ports) continue;

        const cat = t.category || 'other';
        if (DOMAIN_FILTER && !cat.startsWith(DOMAIN_FILTER)) continue;

        COMPONENT_DEFS[t.name] = {
            label: t.glyph || t.name,
            svg: t.svg ? `/${t.svg}` : null,
            w: Number.parseInt(meta.graph_width, 10) || 64,
            h: Number.parseInt(meta.graph_height, 10) || 32,
            ports: parsePorts(meta.ports),
            params: parseParams(meta.params),
            componentType: meta.component_type || null,
            causalityType: meta.causality_type || null,
        };

        if (!sectionMap[cat]) sectionMap[cat] = [];
        sectionMap[cat].push(t.name);
    }

    PALETTE_SECTIONS = Object.entries(sectionMap).map(([cat, items]) => ({
        title: categoryLabel(cat),
        items,
    }));
}

// ---------------------------------------------------------------------------
// Graph state
// ---------------------------------------------------------------------------

let nextComponentId = 0;
let nextNetId = 0;

const graphState = {
    components: [],   // { id, type, x, y, rotation, params: {name: value} }
    nets: [],         // { id, label, connections: [{componentId, portName}] }
};

let interactionMode = 'place';   // 'place' | 'connect' | 'move'
let selectedPaletteType = null;
let selectedComponentId = null;
let selectedNetId = null;
let connectStartPort = null;      // { componentId, portName, x, y }
let dragState = null;             // { componentId, offsetX, offsetY }
let wireDragState = null;         // { netId, axis, mode, wpIndices, startMouse, startValues }

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------

const canvas = document.getElementById('graphCanvas');
const canvasContainer = document.getElementById('canvasContainer');
const componentsLayer = document.getElementById('componentsLayer');
const wiresLayer = document.getElementById('wiresLayer');
const previewLayer = document.getElementById('previewLayer');
const palettePanel = document.getElementById('palettePanel');
const outputContent = document.getElementById('outputContent');
const statusBar = document.getElementById('statusBar');
const gridRect = document.getElementById('gridRect');
const zoomIndicator = document.getElementById('zoomIndicator');

// ---------------------------------------------------------------------------
// Pan & Zoom state
// ---------------------------------------------------------------------------

const viewState = { x: 0, y: 0, zoom: 1 };
const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4;
const ZOOM_STEP = 0.1;
let panState = null; // { startClientX, startClientY, startViewX, startViewY }
let spaceHeld = false;

function applyViewBox() {
    const rect = canvasContainer.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const w = rect.width / viewState.zoom;
    const h = rect.height / viewState.zoom;
    canvas.setAttribute('viewBox', `${viewState.x} ${viewState.y} ${w} ${h}`);
    gridRect.setAttribute('x', viewState.x - 200);
    gridRect.setAttribute('y', viewState.y - 200);
    gridRect.setAttribute('width', w + 400);
    gridRect.setAttribute('height', h + 400);
    if (zoomIndicator) zoomIndicator.textContent = `${Math.round(viewState.zoom * 100)}%`;
}

function zoomAtPoint(clientX, clientY, newZoom) {
    newZoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, newZoom));
    const rect = canvasContainer.getBoundingClientRect();
    const mouseXFrac = (clientX - rect.left) / rect.width;
    const mouseYFrac = (clientY - rect.top) / rect.height;
    const oldW = rect.width / viewState.zoom;
    const oldH = rect.height / viewState.zoom;
    const newW = rect.width / newZoom;
    const newH = rect.height / newZoom;
    viewState.x += (oldW - newW) * mouseXFrac;
    viewState.y += (oldH - newH) * mouseYFrac;
    viewState.zoom = newZoom;
    applyViewBox();
}

function fitToContent() {
    if (graphState.components.length === 0) {
        viewState.x = 0;
        viewState.y = 0;
        viewState.zoom = 1;
        applyViewBox();
        return;
    }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const comp of graphState.components) {
        const { w, h } = getComponentSize(comp);
        minX = Math.min(minX, comp.x);
        minY = Math.min(minY, comp.y);
        maxX = Math.max(maxX, comp.x + w);
        maxY = Math.max(maxY, comp.y + h);
    }
    const pad = 60;
    const contentW = maxX - minX + pad * 2;
    const contentH = maxY - minY + pad * 2;
    const rect = canvasContainer.getBoundingClientRect();
    const scaleX = rect.width / contentW;
    const scaleY = rect.height / contentH;
    viewState.zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, Math.min(scaleX, scaleY)));
    viewState.x = minX - pad;
    viewState.y = minY - pad;
    applyViewBox();
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

function buildPalette() {
    palettePanel.innerHTML = '';
    for (const section of PALETTE_SECTIONS) {
        const div = document.createElement('div');
        div.className = 'palette-section';
        div.innerHTML = `<h3>${section.title}</h3><div class="palette-grid"></div>`;
        const grid = div.querySelector('.palette-grid');
        for (const type of section.items) {
            const def = COMPONENT_DEFS[type];
            if (!def) continue;
            const item = document.createElement('div');
            item.className = 'palette-item' + (type === selectedPaletteType ? ' selected' : '');
            item.dataset.type = type;
            item.innerHTML = `<img src="${def.svg}" alt="${type}"><span>${def.label}</span>`;
            item.addEventListener('click', () => selectPaletteItem(type));
            grid.appendChild(item);
        }
        palettePanel.appendChild(div);
    }
}

function selectPaletteItem(type) {
    selectedPaletteType = type;
    document.querySelectorAll('.palette-item').forEach(el => {
        el.classList.toggle('selected', el.dataset.type === type);
    });
    setMode('place');
    updateStatus();
}

// ---------------------------------------------------------------------------
// Property Panel — edit component parameters
// ---------------------------------------------------------------------------

function updatePropertyPanel() {
    const content = document.getElementById('propertyContent');
    if (!content) return;

    if (!selectedComponentId) {
        content.innerHTML = '<span class="empty-msg">Select a component</span>';
        return;
    }

    const comp = graphState.components.find(c => c.id === selectedComponentId);
    if (!comp) {
        content.innerHTML = '<span class="empty-msg">Select a component</span>';
        return;
    }

    const def = COMPONENT_DEFS[comp.type];
    if (!def || !def.params || def.params.length === 0) {
        content.innerHTML = `<span class="empty-msg">${comp.type} — no parameters</span>`;
        return;
    }

    if (!comp.params) comp.params = {};

    let html = '';
    for (const p of def.params) {
        const val = comp.params[p.name] ?? p.default;
        const step = p.type === 'int' ? '1' : 'any';
        html += `<div class="prop-row">
            <label>${p.name}</label>
            <input type="number" step="${step}" value="${val}"
                   data-comp-id="${comp.id}" data-param="${p.name}" data-ptype="${p.type}">
            <span class="prop-type">${p.type}</span>
        </div>`;
    }
    content.innerHTML = html;

    content.querySelectorAll('input[type="number"]').forEach(input => {
        input.addEventListener('change', (e) => {
            const c = graphState.components.find(c => c.id === e.target.dataset.compId);
            if (!c) return;
            if (!c.params) c.params = {};
            const raw = e.target.value;
            c.params[e.target.dataset.param] = e.target.dataset.ptype === 'int'
                ? parseInt(raw, 10) : parseFloat(raw);
            updateOutput();
        });
    });
}

// ---------------------------------------------------------------------------
// Verify — generic data-driven structural checks from domainConfig.verify_*
// ---------------------------------------------------------------------------

function verifyGraph() {
    const results = [];

    const compTypes = graphState.components.map(c => {
        const def = COMPONENT_DEFS[c.type];
        return { id: c.id, type: c.type, componentType: def?.componentType || c.type };
    });

    // verify_bipartite: "GroupA,B | GroupC,D"
    if (domainConfig.verify_bipartite) {
        const [groupAStr, groupBStr] = domainConfig.verify_bipartite.split('|').map(s => s.trim());
        const groupA = new Set(groupAStr.split(',').map(s => s.trim()));
        const groupB = new Set(groupBStr.split(',').map(s => s.trim()));

        let pass = true;
        let failMsg = '';
        for (const net of graphState.nets) {
            if (net.connections.length < 2) continue;
            const types = net.connections.map(conn => {
                const ct = compTypes.find(c => c.id === conn.componentId);
                return ct ? ct.componentType : '?';
            });
            const hasA = types.some(t => groupA.has(t));
            const hasB = types.some(t => groupB.has(t));
            const allA = types.every(t => groupA.has(t));
            const allB = types.every(t => groupB.has(t));
            if ((hasA && hasB) && !allA && !allB) continue;
            if (allA || allB) {
                pass = false;
                failMsg = `Net ${net.label}: connects components of the same group`;
                break;
            }
        }
        results.push({ rule: 'Bipartite structure', pass, msg: failMsg || 'Arcs correctly cross between groups' });
    }

    // verify_exactly_one: "Type"
    for (const key of Object.keys(domainConfig)) {
        const match = key.match(/^verify_exactly_one(?:_(\w+))?$/);
        if (!match) continue;
        const requiredType = domainConfig[key];
        const count = compTypes.filter(c => c.componentType === requiredType).length;
        results.push({
            rule: `Exactly one ${requiredType}`,
            pass: count === 1,
            msg: count === 1 ? `Found 1 ${requiredType}` : `Found ${count} ${requiredType}(s), expected exactly 1`,
        });
    }

    // verify_requires_type: "Type"
    if (domainConfig.verify_requires_type) {
        const reqType = domainConfig.verify_requires_type;
        const count = compTypes.filter(c => c.componentType === reqType).length;
        results.push({
            rule: `Requires ${reqType}`,
            pass: count >= 1,
            msg: count >= 1 ? `Found ${count} ${reqType}(s)` : `No ${reqType} found — at least one required`,
        });
    }

    // verify_no_isolated: "true"
    if (domainConfig.verify_no_isolated === 'true') {
        const connectedIds = new Set();
        for (const net of graphState.nets) {
            for (const conn of net.connections) connectedIds.add(conn.componentId);
        }
        const isolated = graphState.components.filter(c => !connectedIds.has(c.id));
        results.push({
            rule: 'No isolated components',
            pass: isolated.length === 0,
            msg: isolated.length === 0 ? 'All components connected' : `${isolated.length} isolated component(s): ${isolated.map(c => c.id).join(', ')}`,
        });
    }

    // verify_all_connected: "true" (all nodes reachable from each other via undirected traversal)
    if (domainConfig.verify_all_connected === 'true' && graphState.components.length > 0) {
        const adj = {};
        for (const c of graphState.components) adj[c.id] = new Set();
        for (const net of graphState.nets) {
            const ids = net.connections.map(conn => conn.componentId);
            for (let i = 0; i < ids.length; i++) {
                for (let j = i + 1; j < ids.length; j++) {
                    if (adj[ids[i]]) adj[ids[i]].add(ids[j]);
                    if (adj[ids[j]]) adj[ids[j]].add(ids[i]);
                }
            }
        }
        const visited = new Set();
        const queue = [graphState.components[0].id];
        visited.add(queue[0]);
        while (queue.length > 0) {
            const cur = queue.shift();
            for (const neighbor of (adj[cur] || [])) {
                if (!visited.has(neighbor)) { visited.add(neighbor); queue.push(neighbor); }
            }
        }
        const unreachable = graphState.components.filter(c => !visited.has(c.id));
        results.push({
            rule: 'All connected',
            pass: unreachable.length === 0,
            msg: unreachable.length === 0 ? 'Graph is fully connected' : `${unreachable.length} unreachable component(s)`,
        });
    }

    // verify_causality: "true" (bond graph junction causality)
    if (domainConfig.verify_causality === 'true') {
        let allOk = true;
        let failMsg = '';
        for (const comp of graphState.components) {
            const def = COMPONENT_DEFS[comp.type];
            if (!def || !def.causalityType) continue;
            const ct = def.causalityType;
            if (ct === 'constrained_0junc' || ct === 'constrained_1junc') {
                const connectedNets = graphState.nets.filter(n =>
                    n.connections.some(c => c.componentId === comp.id));
                if (connectedNets.length > 0) {
                    const causalCount = connectedNets.filter(n => n.causal).length;
                    if (causalCount === 0 && connectedNets.length > 1) {
                        allOk = false;
                        failMsg = `${comp.id} (${comp.type}): no causal strokes assigned`;
                    }
                }
            }
        }
        results.push({
            rule: 'Causality constraints',
            pass: allOk,
            msg: allOk ? 'Junction causality OK' : failMsg,
        });
    }

    if (!domainConfig.verify_theory) {
        showVerifyResults(results);
        return;
    }

    const inc = buildIncidenceMatrixJS();
    const payload = {
        domain: domainConfig.verify_theory,
        components: graphState.components.map(c => {
            const def = COMPONENT_DEFS[c.type];
            return {
                type: c.type,
                component_type: def?.componentType || null,
                params: c.params || {},
            };
        }),
        incidence: { entries: inc.entries, v: inc.v, p: inc.p },
        port_labels: inc.port_labels,
    };

    showVerifyResults(results, true);

    fetch('/api/verify_graph', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
    })
    .then(r => r.json())
    .then(resp => {
        const z3Results = (resp.results || []).map(r => ({
            rule: `Z3: ${r.name}`,
            pass: r.passed,
            msg: r.passed ? 'Verified' : (r.error || 'Failed'),
        }));
        showVerifyResults([...results, ...z3Results]);
    })
    .catch(err => {
        showVerifyResults([...results, {
            rule: 'Z3 verification',
            pass: false,
            msg: `Server error: ${err.message}`,
        }]);
    });
}

function showVerifyResults(results, loading) {
    const panel = document.getElementById('verifyResults');
    if (!panel) return;

    const allPass = results.every(r => r.pass);
    let title = allPass ? 'All checks passed' : 'Issues found';
    if (loading) title += ' (Z3 running...)';
    let html = `<h3>${title}</h3>`;
    for (const r of results) {
        const icon = r.pass ? '<span class="vr-pass">&#10003;</span>' : '<span class="vr-fail">&#10007;</span>';
        html += `<div class="vr-item">${icon}<div><strong>${r.rule}</strong><br>${r.msg}</div></div>`;
    }
    if (loading) {
        html += '<div class="vr-item" style="color:#6c757d">Running Z3 verification...</div>';
    }
    html += '<button class="vr-close" id="verifyClose">Close</button>';
    panel.innerHTML = html;
    panel.classList.add('visible');

    document.getElementById('verifyClose').addEventListener('click', () => {
        panel.classList.remove('visible');
    });
}

// ---------------------------------------------------------------------------
// Mode management
// ---------------------------------------------------------------------------

function setMode(mode) {
    interactionMode = mode;
    document.getElementById('modePlace').classList.toggle('active', mode === 'place');
    document.getElementById('modeConnect').classList.toggle('active', mode === 'connect');
    document.getElementById('modeMove').classList.toggle('active', mode === 'move');

    canvas.style.cursor = mode === 'place' ? 'crosshair' : mode === 'connect' ? 'pointer' : 'grab';

    if (mode !== 'connect') {
        connectStartPort = null;
        clearPreview();
        clearPortHighlights();
    }
    updateStatus();
}

function updateStatus() {
    const sel = selectedComponentId ? ` | Selected: ${selectedComponentId}`
               : selectedNetId ? ` | Selected wire: ${selectedNetId}`
               : '';
    const messages = {
        place: `Place \u2014 click canvas to add ${selectedPaletteType}`,
        connect: connectStartPort
            ? `Connect \u2014 click a second port to complete wire (Esc to cancel)`
            : `Connect \u2014 click a port to start a wire`,
        move: `Move \u2014 drag a component to reposition`,
    };
    statusBar.textContent = `${messages[interactionMode] || interactionMode}${sel}`;
}

// ---------------------------------------------------------------------------
// Port geometry helpers (rotation-aware)
// ---------------------------------------------------------------------------

function rotatePoint(rx, ry, rotation) {
    const steps = ((rotation % 360) + 360) % 360 / 90;
    let x = rx, y = ry;
    for (let i = 0; i < steps; i++) {
        const tmp = x;
        x = 1 - y;
        y = tmp;
    }
    return [x, y];
}

function getPortWorldPos(component, portName) {
    const def = COMPONENT_DEFS[component.type];
    const portRel = def.ports[portName];
    if (!portRel) return null;
    const rot = component.rotation || 0;
    const [rx, ry] = rotatePoint(portRel[0], portRel[1], rot);
    const w = (rot % 180 === 0) ? def.w : def.h;
    const h = (rot % 180 === 0) ? def.h : def.w;
    return {
        x: component.x + rx * w,
        y: component.y + ry * h,
    };
}

function getComponentSize(comp) {
    const def = COMPONENT_DEFS[comp.type];
    const rot = comp.rotation || 0;
    return {
        w: (rot % 180 === 0) ? def.w : def.h,
        h: (rot % 180 === 0) ? def.h : def.w,
    };
}

function findPortAt(x, y, excludeComponentId) {
    const threshold = 12 / viewState.zoom;
    let best = null;
    let bestDist = threshold;
    for (const comp of graphState.components) {
        if (comp.id === excludeComponentId) continue;
        const def = COMPONENT_DEFS[comp.type];
        for (const portName of Object.keys(def.ports)) {
            const pos = getPortWorldPos(comp, portName);
            if (!pos) continue;
            const dist = Math.hypot(x - pos.x, y - pos.y);
            if (dist < bestDist) {
                bestDist = dist;
                best = { componentId: comp.id, portName, x: pos.x, y: pos.y };
            }
        }
    }
    return best;
}

// ---------------------------------------------------------------------------
// Port exit direction (determines how wires leave a port)
// ---------------------------------------------------------------------------

const WIRE_STUB = 30;

function getPortExitDir(component, portName) {
    const def = COMPONENT_DEFS[component.type];
    const portRel = def.ports[portName];
    if (!portRel) return { dx: 1, dy: 0 };
    const rot = component.rotation || 0;
    const [rx, ry] = rotatePoint(portRel[0], portRel[1], rot);
    const dists = [
        { dir: { dx: -1, dy: 0 }, d: rx },
        { dir: { dx: 1, dy: 0 }, d: 1 - rx },
        { dir: { dx: 0, dy: -1 }, d: ry },
        { dir: { dx: 0, dy: 1 }, d: 1 - ry },
    ];
    dists.sort((a, b) => a.d - b.d);
    return dists[0].dir;
}

function computeDefaultWaypoints(compA, portNameA, compB, portNameB) {
    const a = getPortWorldPos(compA, portNameA);
    const b = getPortWorldPos(compB, portNameB);
    if (!a || !b) return [];

    if (domainConfig.routing_mode === 'direct') return [];

    const dA = getPortExitDir(compA, portNameA);
    const dB = getPortExitDir(compB, portNameB);
    const aIsH = dA.dy === 0;
    const bIsH = dB.dy === 0;

    if (aIsH && bIsH) {
        const midX = snapToGrid((a.x + dA.dx * WIRE_STUB + b.x + dB.dx * WIRE_STUB) / 2);
        return [{ x: midX, y: a.y }, { x: midX, y: b.y }];
    }
    if (!aIsH && !bIsH) {
        const midY = snapToGrid((a.y + dA.dy * WIRE_STUB + b.y + dB.dy * WIRE_STUB) / 2);
        return [{ x: a.x, y: midY }, { x: b.x, y: midY }];
    }
    if (aIsH) {
        return [{ x: b.x, y: a.y }];
    }
    return [{ x: a.x, y: b.y }];
}

// ---------------------------------------------------------------------------
// Wire path geometry
// ---------------------------------------------------------------------------

function distToSegment(px, py, x1, y1, x2, y2) {
    const dx = x2 - x1, dy = y2 - y1;
    const lenSq = dx * dx + dy * dy;
    if (lenSq === 0) return Math.hypot(px - x1, py - y1);
    const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / lenSq));
    return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}

function getNetPathPoints(net) {
    if (net.connections.length < 2) return [];
    const positions = net.connections.map(conn => {
        const comp = graphState.components.find(c => c.id === conn.componentId);
        if (!comp) return null;
        return getPortWorldPos(comp, conn.portName);
    });
    if (!positions[0] || !positions[1]) return [];
    const wps = net.waypoints || [];
    return [positions[0], ...wps, positions[1]];
}

function projectOntoPolyline(pt, polyline) {
    let bestDist = Infinity, bestPt = null, bestSegIdx = -1;
    for (let i = 0; i < polyline.length - 1; i++) {
        const p1 = polyline[i], p2 = polyline[i + 1];
        const dx = p2.x - p1.x, dy = p2.y - p1.y;
        const lenSq = dx * dx + dy * dy;
        let proj;
        if (lenSq === 0) {
            proj = { x: p1.x, y: p1.y };
        } else {
            const t = Math.max(0, Math.min(1, ((pt.x - p1.x) * dx + (pt.y - p1.y) * dy) / lenSq));
            proj = { x: p1.x + t * dx, y: p1.y + t * dy };
        }
        const d = Math.hypot(pt.x - proj.x, pt.y - proj.y);
        if (d < bestDist) {
            bestDist = d;
            bestPt = proj;
            bestSegIdx = i;
        }
    }
    return { point: bestPt, segIdx: bestSegIdx, dist: bestDist };
}

function computeTrunkBranch(net) {
    if (net.connections.length < 3) return null;
    const connData = net.connections.map(conn => {
        const comp = graphState.components.find(c => c.id === conn.componentId);
        if (!comp) return null;
        const pos = getPortWorldPos(comp, conn.portName);
        const dir = getPortExitDir(comp, conn.portName);
        return pos ? { conn, comp, pos, dir, isH: dir.dy === 0 } : null;
    }).filter(Boolean);
    if (connData.length < 3) return null;

    let maxDist = -1, trunkA = 0, trunkB = 1;
    for (let i = 0; i < connData.length; i++) {
        for (let j = i + 1; j < connData.length; j++) {
            const d = Math.hypot(connData[i].pos.x - connData[j].pos.x, connData[i].pos.y - connData[j].pos.y);
            if (d > maxDist) { maxDist = d; trunkA = i; trunkB = j; }
        }
    }

    const cA = connData[trunkA], cB = connData[trunkB];
    const trunkWps = computeDefaultWaypoints(cA.comp, cA.conn.portName, cB.comp, cB.conn.portName);
    const trunkPath = [cA.pos, ...trunkWps, cB.pos];

    const branches = [];
    for (let i = 0; i < connData.length; i++) {
        if (i === trunkA || i === trunkB) continue;
        const br = connData[i];
        const proj = projectOntoPolyline(br.pos, trunkPath);
        const jx = snapToGrid(proj.point.x);
        const jy = snapToGrid(proj.point.y);
        const junctionPt = { x: jx, y: jy };

        let leg;
        if (Math.abs(br.pos.x - jx) < 2 && Math.abs(br.pos.y - jy) < 2) {
            leg = [br.pos, junctionPt];
        } else if (br.isH) {
            const stubX = br.pos.x + br.dir.dx * WIRE_STUB;
            if (Math.abs(jy - br.pos.y) < 2) {
                leg = [br.pos, junctionPt];
            } else {
                leg = [br.pos, { x: stubX, y: br.pos.y }, { x: stubX, y: jy }, junctionPt];
            }
        } else {
            const stubY = br.pos.y + br.dir.dy * WIRE_STUB;
            if (Math.abs(jx - br.pos.x) < 2) {
                leg = [br.pos, junctionPt];
            } else {
                leg = [br.pos, { x: br.pos.x, y: stubY }, { x: jx, y: stubY }, junctionPt];
            }
        }
        branches.push({ leg, junction: junctionPt });
    }

    return { trunkPath, branches };
}

function getMultiNetLegs(net) {
    if (net.connections.length < 3) return [];
    if (domainConfig.multi_port_strategy === 'trunk_branch') {
        const tb = computeTrunkBranch(net);
        if (!tb) return [];
        const legs = [tb.trunkPath];
        for (const br of tb.branches) legs.push(br.leg);
        return { legs, junctions: tb.branches.map(b => b.junction), isTrunkBranch: true };
    }
    const connData = net.connections.map(conn => {
        const comp = graphState.components.find(c => c.id === conn.componentId);
        if (!comp) return null;
        const pos = getPortWorldPos(comp, conn.portName);
        const dir = getPortExitDir(comp, conn.portName);
        return pos ? { pos, dir, isH: dir.dy === 0 } : null;
    }).filter(Boolean);
    if (connData.length < 3) return [];
    const jx = snapToGrid(connData.reduce((s, c) => s + c.pos.x, 0) / connData.length);
    const jy = snapToGrid(connData.reduce((s, c) => s + c.pos.y, 0) / connData.length);
    const junction = { x: jx, y: jy };
    const legs = connData.map(({ pos, dir, isH }) => {
        if (Math.abs(pos.x - jx) < 2 && Math.abs(pos.y - jy) < 2) {
            return [pos, junction];
        }
        if (isH) {
            const stubX = pos.x + dir.dx * WIRE_STUB;
            return [pos, { x: stubX, y: pos.y }, { x: stubX, y: jy }, junction];
        }
        const stubY = pos.y + dir.dy * WIRE_STUB;
        return [pos, { x: pos.x, y: stubY }, { x: jx, y: stubY }, junction];
    });
    return { legs, junctions: [junction], isTrunkBranch: false };
}

function buildSvgPathD(points) {
    if (points.length < 2) return '';
    let d = `M${points[0].x},${points[0].y}`;
    for (let i = 1; i < points.length; i++) d += ` L${points[i].x},${points[i].y}`;
    return d;
}

const DECORATION_MARKER_ID = {
    arrow: 'arrowhead',
    half_arrow: 'half_arrow',
    inhibitor: 'inhibitor',
    causal_bar: 'causal_bar',
};

function applyEdgeMarkers(pathEl, isSelected, net) {
    const dec = domainConfig.edge_decoration;
    const suffix = isSelected ? '-selected' : '';

    if (dec !== 'none') {
        const markerId = DECORATION_MARKER_ID[dec] || dec;
        pathEl.setAttribute('marker-end', `url(#${markerId}${suffix})`);
    }

    if (net && net.causal) {
        const barId = `causal_bar${suffix}`;
        if (net.causal === 'start') {
            pathEl.setAttribute('marker-start', `url(#${barId})`);
        } else if (net.causal === 'end') {
            pathEl.setAttribute('marker-end', `url(#causal_bar${suffix})`);
        }
    }
}

function segIsHorizontal(p1, p2) {
    return Math.abs(p1.y - p2.y) <= Math.abs(p1.x - p2.x);
}

function findSegmentAt(x, y, threshold = 8 / viewState.zoom) {
    for (const net of graphState.nets) {
        if (net.connections.length === 2) {
            const pts = getNetPathPoints(net);
            for (let i = 0; i < pts.length - 1; i++) {
                if (distToSegment(x, y, pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y) < threshold) {
                    const isH = segIsHorizontal(pts[i], pts[i + 1]);
                    return { net, segIndex: i, axis: isH ? 'y' : 'x', canDrag: true };
                }
            }
        } else {
            const multiResult = getMultiNetLegs(net);
            if (multiResult && multiResult.legs) {
                for (const leg of multiResult.legs) {
                    for (let i = 0; i < leg.length - 1; i++) {
                        if (distToSegment(x, y, leg[i].x, leg[i].y, leg[i + 1].x, leg[i + 1].y) < threshold) {
                            return { net, segIndex: -1, axis: null, canDrag: false };
                        }
                    }
                }
            }
        }
    }
    return null;
}

function findWaypointAt(x, y, threshold = 10 / viewState.zoom) {
    for (const net of graphState.nets) {
        const wps = net.waypoints;
        if (!wps) continue;
        for (let i = 0; i < wps.length; i++) {
            if (Math.hypot(x - wps[i].x, y - wps[i].y) < threshold) {
                return { net, wpIndex: i };
            }
        }
    }
    return null;
}

function findNetAt(x, y) {
    const seg = findSegmentAt(x, y);
    return seg ? seg.net : null;
}

function collapseZeroLengthSegments(net) {
    const wps = net.waypoints;
    if (!wps || wps.length < 2) return;
    let i = 0;
    while (i < wps.length - 1) {
        if (Math.abs(wps[i].x - wps[i + 1].x) < 2 && Math.abs(wps[i].y - wps[i + 1].y) < 2) {
            wps.splice(i, 2);
            if (i > 0) i--;
        } else {
            i++;
        }
    }
}

function collapseCollinearSegments(net) {
    if (!net.waypoints || net.waypoints.length === 0) return;
    const pts = getNetPathPoints(net);
    if (pts.length < 3) return;
    const keep = [];
    for (let i = 0; i < net.waypoints.length; i++) {
        const prev = i === 0 ? pts[0] : net.waypoints[i - 1];
        const curr = net.waypoints[i];
        const next = i === net.waypoints.length - 1 ? pts[pts.length - 1] : net.waypoints[i + 1];
        const sameX = Math.abs(prev.x - curr.x) < 2 && Math.abs(curr.x - next.x) < 2;
        const sameY = Math.abs(prev.y - curr.y) < 2 && Math.abs(curr.y - next.y) < 2;
        if (!sameX && !sameY) keep.push(curr);
    }
    net.waypoints = keep;
}

function cleanupNet(net) {
    collapseZeroLengthSegments(net);
    collapseCollinearSegments(net);
}

function autoRouteNet(net) {
    if (net.connections.length === 2) {
        const cA = graphState.components.find(c => c.id === net.connections[0].componentId);
        const cB = graphState.components.find(c => c.id === net.connections[1].componentId);
        if (cA && cB) {
            net.waypoints = computeDefaultWaypoints(cA, net.connections[0].portName, cB, net.connections[1].portName);
        }
    } else {
        net.waypoints = [];
    }
}

function autoRouteAllNets() {
    for (const net of graphState.nets) {
        autoRouteNet(net);
    }
    renderAll();
    updateOutput();
}

function findComponentAt(x, y) {
    for (let i = graphState.components.length - 1; i >= 0; i--) {
        const comp = graphState.components[i];
        const { w, h } = getComponentSize(comp);
        if (x >= comp.x && x <= comp.x + w && y >= comp.y && y <= comp.y + h) {
            return comp;
        }
    }
    return null;
}

// ---------------------------------------------------------------------------
// Snap to grid
// ---------------------------------------------------------------------------

function snapToGrid(val, gridSize = 20) {
    return Math.round(val / gridSize) * gridSize;
}

// ---------------------------------------------------------------------------
// Place component
// ---------------------------------------------------------------------------

function placeComponent(type, x, y) {
    const def = COMPONENT_DEFS[type];
    if (!def) return;
    const cx = snapToGrid(x - def.w / 2);
    const cy = snapToGrid(y - def.h / 2);
    const params = {};
    for (const p of def.params) params[p.name] = p.default;
    const comp = { id: `c${nextComponentId++}`, type, x: cx, y: cy, rotation: 0, params };
    graphState.components.push(comp);
    selectedComponentId = comp.id;
    renderAll();
    updatePropertyPanel();
    updateOutput();
    updateStatus();
}

// ---------------------------------------------------------------------------
// Connect ports
// ---------------------------------------------------------------------------

function connectPorts(portA, portB) {
    const netA = findNetForPort(portA.componentId, portA.portName);
    const netB = findNetForPort(portB.componentId, portB.portName);

    if (netA && netB && netA.id === netB.id) return;

    function autoWaypoints(net) {
        if (net.connections.length !== 2) { net.waypoints = []; return; }
        const cA = graphState.components.find(c => c.id === net.connections[0].componentId);
        const cB = graphState.components.find(c => c.id === net.connections[1].componentId);
        net.waypoints = (cA && cB)
            ? computeDefaultWaypoints(cA, net.connections[0].portName, cB, net.connections[1].portName)
            : [];
    }

    if (netA && netB) {
        for (const conn of netB.connections) {
            if (!netA.connections.some(c => c.componentId === conn.componentId && c.portName === conn.portName)) {
                netA.connections.push(conn);
            }
        }
        autoWaypoints(netA);
        graphState.nets = graphState.nets.filter(n => n.id !== netB.id);
    } else if (netA) {
        netA.connections.push({ componentId: portB.componentId, portName: portB.portName });
        autoWaypoints(netA);
    } else if (netB) {
        netB.connections.push({ componentId: portA.componentId, portName: portA.portName });
        autoWaypoints(netB);
    } else {
        const compA = graphState.components.find(c => c.id === portA.componentId);
        const compB = graphState.components.find(c => c.id === portB.componentId);
        const wps = (compA && compB)
            ? computeDefaultWaypoints(compA, portA.portName, compB, portB.portName)
            : [];
        graphState.nets.push({
            id: `n${nextNetId++}`,
            label: `n${nextNetId - 1}`,
            connections: [
                { componentId: portA.componentId, portName: portA.portName },
                { componentId: portB.componentId, portName: portB.portName },
            ],
            waypoints: wps,
        });
    }

    renderAll();
    updateOutput();
}

function findNetForPort(componentId, portName) {
    return graphState.nets.find(n =>
        n.connections.some(c => c.componentId === componentId && c.portName === portName)
    );
}

// ---------------------------------------------------------------------------
// Delete / Rotate / Clear
// ---------------------------------------------------------------------------

function clearSelection() {
    selectedComponentId = null;
    selectedNetId = null;
    updatePropertyPanel();
}

function deleteSelected() {
    if (selectedNetId) {
        graphState.nets = graphState.nets.filter(n => n.id !== selectedNetId);
        selectedNetId = null;
        renderAll();
        updateOutput();
        updateStatus();
        return;
    }
    if (selectedComponentId) {
        graphState.nets = graphState.nets.map(net => ({
            ...net,
            connections: net.connections.filter(c => c.componentId !== selectedComponentId)
        })).filter(net => net.connections.length >= 2);

        graphState.components = graphState.components.filter(c => c.id !== selectedComponentId);
        selectedComponentId = null;
        renderAll();
        updateOutput();
        updateStatus();
        return;
    }
    statusBar.textContent = 'Nothing selected \u2014 click a component or wire first';
    setTimeout(updateStatus, 2000);
}

function rotateSelected() {
    if (!selectedComponentId) {
        statusBar.textContent = 'Nothing selected \u2014 click a component first';
        setTimeout(updateStatus, 2000);
        return;
    }
    const comp = graphState.components.find(c => c.id === selectedComponentId);
    if (!comp) return;
    comp.rotation = ((comp.rotation || 0) + 90) % 360;
    renderAll();
    updateOutput();
}

function toggleCausalStroke() {
    if (!selectedNetId) {
        statusBar.textContent = 'No wire selected \u2014 click a wire first, then press K';
        setTimeout(updateStatus, 2000);
        return;
    }
    const net = graphState.nets.find(n => n.id === selectedNetId);
    if (!net) return;
    if (!net.causal) net.causal = 'end';
    else if (net.causal === 'end') net.causal = 'start';
    else net.causal = null;
    statusBar.textContent = net.causal ? `Causal stroke: ${net.causal}` : 'Causal stroke removed';
    setTimeout(updateStatus, 2000);
    renderAll();
    updateOutput();
}

function clearAll() {
    graphState.components = [];
    graphState.nets = [];
    nextComponentId = 0;
    nextNetId = 0;
    selectedComponentId = null;
    connectStartPort = null;
    renderAll();
    updateOutput();
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

function renderAll() {
    renderComponents();
    renderWires();
}

function renderComponents() {
    componentsLayer.innerHTML = '';
    for (const comp of graphState.components) {
        const def = COMPONENT_DEFS[comp.type];
        const rot = comp.rotation || 0;
        const { w, h } = getComponentSize(comp);
        const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
        g.setAttribute('class', 'component-group' + (comp.id === selectedComponentId ? ' selected' : ''));
        g.setAttribute('data-id', comp.id);
        g.setAttribute('transform', `translate(${comp.x}, ${comp.y})`);

        const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
        rect.setAttribute('class', 'component-outline');
        rect.setAttribute('x', '-4');
        rect.setAttribute('y', '-4');
        rect.setAttribute('width', w + 8);
        rect.setAttribute('height', h + 8);
        rect.setAttribute('rx', '3');
        rect.setAttribute('fill', 'none');
        rect.setAttribute('stroke', 'transparent');
        rect.setAttribute('stroke-width', '1.5');
        g.appendChild(rect);

        const imgGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
        if (rot !== 0) {
            imgGroup.setAttribute('transform',
                `translate(${w / 2}, ${h / 2}) rotate(${rot}) translate(${-def.w / 2}, ${-def.h / 2})`
            );
        }
        const img = document.createElementNS('http://www.w3.org/2000/svg', 'image');
        img.setAttribute('href', def.svg);
        img.setAttribute('width', def.w);
        img.setAttribute('height', def.h);
        imgGroup.appendChild(img);
        g.appendChild(imgGroup);

        const label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        label.setAttribute('class', 'component-label');
        label.setAttribute('x', w / 2);
        label.setAttribute('y', h + 14);
        label.textContent = def.label;
        g.appendChild(label);

        for (const portName of Object.keys(def.ports)) {
            const pos = getPortWorldPos(comp, portName);
            if (!pos) continue;
            const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
            circle.setAttribute('class', 'port-circle');
            circle.setAttribute('cx', pos.x - comp.x);
            circle.setAttribute('cy', pos.y - comp.y);
            circle.setAttribute('r', '5');
            circle.dataset.componentId = comp.id;
            circle.dataset.portName = portName;

            const title = document.createElementNS('http://www.w3.org/2000/svg', 'title');
            title.textContent = portName;
            circle.appendChild(title);

            g.appendChild(circle);
        }

        componentsLayer.appendChild(g);
    }
}

function renderWires() {
    wiresLayer.innerHTML = '';
    for (const net of graphState.nets) {
        if (net.connections.length < 2) continue;

        const isSel = net.id === selectedNetId;
        const wireClass = isSel ? 'wire wire-selected' : 'wire';

        if (net.connections.length === 2) {
            const pts = getNetPathPoints(net);
            if (pts.length < 2) continue;

            const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
            path.setAttribute('class', wireClass);
            path.setAttribute('d', buildSvgPathD(pts));
            path.dataset.netId = net.id;
            applyEdgeMarkers(path, isSel, net);
            wiresLayer.appendChild(path);

            for (let i = 0; i < pts.length - 1; i++) {
                const isH = segIsHorizontal(pts[i], pts[i + 1]);
                const hitLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
                hitLine.setAttribute('x1', pts[i].x);
                hitLine.setAttribute('y1', pts[i].y);
                hitLine.setAttribute('x2', pts[i + 1].x);
                hitLine.setAttribute('y2', pts[i + 1].y);
                const dragClass = domainConfig.routing_mode === 'direct' ? '' : (isH ? 'draggable-h' : 'draggable-v');
                hitLine.setAttribute('class', `wire-seg ${dragClass}`);
                hitLine.dataset.netId = net.id;
                hitLine.dataset.segIndex = i;
                wiresLayer.appendChild(hitLine);
            }

            const wps = net.waypoints || [];
            for (let i = 0; i < wps.length; i++) {
                const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                dot.setAttribute('class', 'wire-bend' + (isSel ? ' visible' : ''));
                dot.setAttribute('cx', wps[i].x);
                dot.setAttribute('cy', wps[i].y);
                dot.setAttribute('r', '4');
                dot.dataset.netId = net.id;
                dot.dataset.wpIndex = i;
                wiresLayer.appendChild(dot);
            }

            const labelPt = pts[0];
            const labelEl = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            labelEl.setAttribute('class', 'net-label');
            labelEl.setAttribute('x', labelPt.x + 8);
            labelEl.setAttribute('y', labelPt.y - 8);
            labelEl.textContent = net.label;
            wiresLayer.appendChild(labelEl);
        } else {
            const multiResult = getMultiNetLegs(net);
            if (multiResult && multiResult.legs) {
                for (const leg of multiResult.legs) {
                    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
                    path.setAttribute('class', wireClass);
                    path.setAttribute('d', buildSvgPathD(leg));
                    path.dataset.netId = net.id;
                    applyEdgeMarkers(path, isSel, net);
                    wiresLayer.appendChild(path);
                }
                if (domainConfig.junction_style === 'dot') {
                    for (const jpt of multiResult.junctions) {
                        const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                        dot.setAttribute('cx', jpt.x);
                        dot.setAttribute('cy', jpt.y);
                        dot.setAttribute('r', '4');
                        dot.setAttribute('fill', isSel ? '#5568d3' : '#2c3e50');
                        wiresLayer.appendChild(dot);
                    }
                }
            }

            const firstPos = (() => {
                const conn = net.connections[0];
                const comp = graphState.components.find(c => c.id === conn.componentId);
                return comp ? getPortWorldPos(comp, conn.portName) : null;
            })();
            if (firstPos) {
                const labelEl = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                labelEl.setAttribute('class', 'net-label');
                labelEl.setAttribute('x', firstPos.x + 8);
                labelEl.setAttribute('y', firstPos.y - 8);
                labelEl.textContent = net.label;
                wiresLayer.appendChild(labelEl);
            }
        }
    }
}

function clearPreview() {
    previewLayer.innerHTML = '';
}

function clearPortHighlights() {
    document.querySelectorAll('.port-circle.highlight').forEach(el => el.classList.remove('highlight'));
}

// ---------------------------------------------------------------------------
// Canvas event handling
// ---------------------------------------------------------------------------

function getSVGPoint(e) {
    const pt = canvas.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const svgPt = pt.matrixTransform(canvas.getScreenCTM().inverse());
    return { x: svgPt.x, y: svgPt.y };
}

canvas.addEventListener('mousedown', (e) => {
    // Pan: middle-click or Space+left-click
    if (e.button === 1 || (e.button === 0 && spaceHeld)) {
        e.preventDefault();
        panState = {
            startClientX: e.clientX,
            startClientY: e.clientY,
            startViewX: viewState.x,
            startViewY: viewState.y,
        };
        canvas.style.cursor = 'grabbing';
        return;
    }

    const pt = getSVGPoint(e);
    const clickedComp = findComponentAt(pt.x, pt.y);

    // Connect mode: handle port clicks; if no port hit, fall through to select/drag
    if (interactionMode === 'connect') {
        const port = findPortAt(pt.x, pt.y);
        if (port) {
            if (!connectStartPort) {
                connectStartPort = port;
                const circle = document.querySelector(
                    `.port-circle[data-component-id="${port.componentId}"][data-port-name="${port.portName}"]`
                );
                if (circle) circle.classList.add('highlight');
                updateStatus();
            } else {
                if (port.componentId === connectStartPort.componentId && port.portName === connectStartPort.portName) return;
                connectPorts(connectStartPort, port);
                connectStartPort = null;
                clearPreview();
                clearPortHighlights();
                updateStatus();
            }
            return;
        }
        // No port clicked — fall through to selection/drag below
    }

    // Clicking a component selects it and starts drag (works in ANY mode)
    if (clickedComp) {
        clearSelection();
        selectedComponentId = clickedComp.id;
        dragState = { componentId: clickedComp.id, offsetX: pt.x - clickedComp.x, offsetY: pt.y - clickedComp.y };
        canvas.style.cursor = 'grabbing';
        renderAll();
        updatePropertyPanel();
        updateStatus();
        return;
    }

    // Clicking a waypoint dot: treat as dragging its outgoing segment (orthogonal only)
    const wpHit = findWaypointAt(pt.x, pt.y);
    if (wpHit && wpHit.net.connections.length === 2 && domainConfig.routing_mode !== 'direct') {
        clearSelection();
        selectedNetId = wpHit.net.id;
        const net = wpHit.net;
        const wps = net.waypoints || [];
        const wi = wpHit.wpIndex;
        const outSegIdx = wi + 1;
        const pts = getNetPathPoints(net);
        const isH = segIsHorizontal(pts[outSegIdx], pts[outSegIdx + 1]);
        const axis = isH ? 'y' : 'x';
        const indices = [wi];
        if (wi + 1 < wps.length) indices.push(wi + 1);
        wireDragState = {
            netId: net.id, axis, mode: 'mid',
            wpIndices: indices,
            startMouse: axis === 'x' ? pt.x : pt.y,
            startValues: indices.map(idx => axis === 'x' ? wps[idx].x : wps[idx].y),
        };
        canvas.style.cursor = axis === 'x' ? 'ew-resize' : 'ns-resize';
        renderAll();
        updateStatus();
        return;
    }

    // Clicking a wire: start segment drag (orthogonal only)
    const seg = findSegmentAt(pt.x, pt.y);
    if (seg) {
        clearSelection();
        selectedNetId = seg.net.id;
        if (seg.canDrag && seg.net.connections.length === 2 && domainConfig.routing_mode !== 'direct') {
            const wps = seg.net.waypoints || [];
            const pts = getNetPathPoints(seg.net);
            const i = seg.segIndex;
            const isFirst = (i === 0);
            const isLast = (i === pts.length - 2);
            const isMid = !isFirst && !isLast;

            if (isMid) {
                const wpI = i - 1, wpJ = i;
                const indices = [];
                if (wpI >= 0 && wpI < wps.length) indices.push(wpI);
                if (wpJ >= 0 && wpJ < wps.length && wpJ !== wpI) indices.push(wpJ);
                wireDragState = {
                    netId: seg.net.id, axis: seg.axis, mode: 'mid',
                    wpIndices: indices,
                    startMouse: seg.axis === 'x' ? pt.x : pt.y,
                    startValues: indices.map(idx => seg.axis === 'x' ? wps[idx].x : wps[idx].y),
                };
            } else {
                wireDragState = {
                    netId: seg.net.id, axis: seg.axis,
                    mode: isFirst ? 'first' : 'last',
                    startMouse: seg.axis === 'x' ? pt.x : pt.y,
                };
            }
            canvas.style.cursor = seg.axis === 'x' ? 'ew-resize' : 'ns-resize';
        }
        renderAll();
        updateStatus();
        return;
    }

    // Clicking empty canvas in place mode adds a component
    if (interactionMode === 'place') {
        placeComponent(selectedPaletteType, pt.x, pt.y);
        return;
    }

    // Clicking empty canvas deselects
    clearSelection();
    renderAll();
    updateStatus();
});

canvas.addEventListener('mousemove', (e) => {
    if (panState) {
        const dx = (e.clientX - panState.startClientX) / viewState.zoom;
        const dy = (e.clientY - panState.startClientY) / viewState.zoom;
        viewState.x = panState.startViewX - dx;
        viewState.y = panState.startViewY - dy;
        applyViewBox();
        return;
    }

    const pt = getSVGPoint(e);

    if (interactionMode === 'connect' && connectStartPort) {
        clearPreview();
        const comp = graphState.components.find(c => c.id === connectStartPort.componentId);
        let previewPts;
        if (comp) {
            const a = { x: connectStartPort.x, y: connectStartPort.y };
            const b = { x: pt.x, y: pt.y };

            if (domainConfig.routing_mode === 'direct') {
                previewPts = [a, b];
            } else {
                const dA = getPortExitDir(comp, connectStartPort.portName);
                const targetPort = findPortAt(pt.x, pt.y, connectStartPort.componentId);
                const targetComp = targetPort ? graphState.components.find(c => c.id === targetPort.componentId) : null;
                const dB = targetComp ? getPortExitDir(targetComp, targetPort.portName) : null;
                const aIsH = dA.dy === 0;
                const bIsH = dB ? dB.dy === 0 : aIsH;

                if (aIsH && bIsH) {
                    const stubB = dB ? b.x + dB.dx * WIRE_STUB : b.x;
                    const midX = snapToGrid((a.x + dA.dx * WIRE_STUB + stubB) / 2);
                    previewPts = [a, { x: midX, y: a.y }, { x: midX, y: b.y }, b];
                } else if (!aIsH && !bIsH) {
                    const stubB = dB ? b.y + dB.dy * WIRE_STUB : b.y;
                    const midY = snapToGrid((a.y + dA.dy * WIRE_STUB + stubB) / 2);
                    previewPts = [a, { x: a.x, y: midY }, { x: b.x, y: midY }, b];
                } else if (aIsH) {
                    previewPts = [a, { x: b.x, y: a.y }, b];
                } else {
                    previewPts = [a, { x: a.x, y: b.y }, b];
                }
            }
        } else {
            previewPts = [{ x: connectStartPort.x, y: connectStartPort.y }, { x: pt.x, y: pt.y }];
        }
        const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        path.setAttribute('class', 'wire-preview');
        path.setAttribute('d', buildSvgPathD(previewPts));
        applyEdgeMarkers(path, false);
        previewLayer.appendChild(path);
    }

    if (wireDragState) {
        const net = graphState.nets.find(n => n.id === wireDragState.netId);
        if (!net || !net.waypoints) { /* skip */ }
        else if (wireDragState.mode === 'mid') {
            const mouseVal = wireDragState.axis === 'x' ? pt.x : pt.y;
            const delta = mouseVal - wireDragState.startMouse;
            for (let k = 0; k < wireDragState.wpIndices.length; k++) {
                const wi = wireDragState.wpIndices[k];
                const newVal = snapToGrid(wireDragState.startValues[k] + delta);
                if (wireDragState.axis === 'x') net.waypoints[wi].x = newVal;
                else net.waypoints[wi].y = newVal;
            }
            renderAll();
        } else if (wireDragState.mode === 'first' || wireDragState.mode === 'last') {
            const mouseVal = wireDragState.axis === 'x' ? pt.x : pt.y;
            const snapped = snapToGrid(mouseVal);
            if (!wireDragState.inserted) {
                const pts = getNetPathPoints(net);
                if (wireDragState.mode === 'first') {
                    const port = pts[0];
                    const wp0 = net.waypoints[0];
                    if (wireDragState.axis === 'y') {
                        net.waypoints.splice(0, 0, { x: port.x, y: snapped });
                        if (wp0) wp0.y = snapped;
                    } else {
                        net.waypoints.splice(0, 0, { x: snapped, y: port.y });
                        if (wp0) wp0.x = snapped;
                    }
                } else {
                    const port = pts[pts.length - 1];
                    const wpLast = net.waypoints[net.waypoints.length - 1];
                    if (wireDragState.axis === 'y') {
                        net.waypoints.push({ x: port.x, y: snapped });
                        if (wpLast) wpLast.y = snapped;
                    } else {
                        net.waypoints.push({ x: snapped, y: port.y });
                        if (wpLast) wpLast.x = snapped;
                    }
                }
                const newWps = net.waypoints;
                const idxA = wireDragState.mode === 'first' ? 0 : newWps.length - 2;
                const idxB = wireDragState.mode === 'first' ? 1 : newWps.length - 1;
                wireDragState.inserted = true;
                wireDragState.mode = 'mid';
                wireDragState.wpIndices = [idxA, idxB];
                wireDragState.startMouse = mouseVal;
                wireDragState.startValues = [
                    wireDragState.axis === 'x' ? newWps[idxA].x : newWps[idxA].y,
                    wireDragState.axis === 'x' ? newWps[idxB].x : newWps[idxB].y,
                ];
            }
            renderAll();
        }
    }

    if (dragState) {
        const comp = graphState.components.find(c => c.id === dragState.componentId);
        if (comp) {
            comp.x = snapToGrid(pt.x - dragState.offsetX);
            comp.y = snapToGrid(pt.y - dragState.offsetY);
            for (const net of graphState.nets) {
                if (net.connections.some(c => c.componentId === dragState.componentId)) {
                    autoRouteNet(net);
                }
            }
            renderAll();
        }
    }
});

canvas.addEventListener('mouseup', () => {
    if (panState) {
        panState = null;
        const cursors = { place: 'crosshair', connect: 'pointer', move: 'grab' };
        canvas.style.cursor = cursors[interactionMode] || 'default';
        return;
    }
    const hadDrag = wireDragState || dragState;
    if (wireDragState) {
        const net = graphState.nets.find(n => n.id === wireDragState.netId);
        if (net) cleanupNet(net);
    }
    wireDragState = null;
    dragState = null;
    if (hadDrag) {
        const cursors = { place: 'crosshair', connect: 'pointer', move: 'grab' };
        canvas.style.cursor = cursors[interactionMode] || 'default';
        renderAll();
        updateOutput();
    }
});

canvas.addEventListener('dblclick', (e) => {
    const pt = getSVGPoint(e);

    // Double-click a waypoint dot: remove it and its neighbor to merge segments (orthogonal only)
    const wpHit = findWaypointAt(pt.x, pt.y);
    if (wpHit && wpHit.net.connections.length === 2 && domainConfig.routing_mode !== 'direct') {
        const net = wpHit.net;
        const wps = net.waypoints;
        if (wps.length <= 2) {
            const connA = net.connections[0], connB = net.connections[1];
            const compA = graphState.components.find(c => c.id === connA.componentId);
            const compB = graphState.components.find(c => c.id === connB.componentId);
            if (compA && compB) {
                net.waypoints = computeDefaultWaypoints(compA, connA.portName, compB, connB.portName);
            }
        } else {
            const idx = wpHit.wpIndex;
            const removeStart = Math.max(0, idx - 1);
            const removeCount = Math.min(2, wps.length - removeStart);
            wps.splice(removeStart, removeCount);
        }
        cleanupNet(net);
        renderAll();
        updateOutput();
        return;
    }

    // Double-click a segment: insert a waypoint pair to split it into 3 (orthogonal only)
    const seg = findSegmentAt(pt.x, pt.y);
    if (seg && seg.net.connections.length === 2 && domainConfig.routing_mode !== 'direct') {
        const net = seg.net;
        const pts = getNetPathPoints(net);
        const si = seg.segIndex;
        const p1 = pts[si], p2 = pts[si + 1];
        const isH = segIsHorizontal(p1, p2);

        if (isH) {
            const midX = snapToGrid((p1.x + p2.x) / 2);
            net.waypoints.splice(si, 0,
                { x: midX, y: p1.y },
                { x: midX, y: p1.y }
            );
        } else {
            const midY = snapToGrid((p1.y + p2.y) / 2);
            net.waypoints.splice(si, 0,
                { x: p1.x, y: midY },
                { x: p1.x, y: midY }
            );
        }
        cleanupNet(net);
        selectedNetId = net.id;
        renderAll();
        updateOutput();
    }
});

canvas.addEventListener('contextmenu', (e) => e.preventDefault());

// ---------------------------------------------------------------------------
// Wheel zoom
// ---------------------------------------------------------------------------

canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    const delta = -Math.sign(e.deltaY) * ZOOM_STEP;
    zoomAtPoint(e.clientX, e.clientY, viewState.zoom + delta);
}, { passive: false });

// Prevent browser zoom on Ctrl+wheel over the canvas container
canvasContainer.addEventListener('wheel', (e) => {
    if (e.ctrlKey) e.preventDefault();
}, { passive: false });

// ---------------------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------------------

document.addEventListener('keydown', (e) => {
    if (e.key === ' ') { spaceHeld = true; canvas.style.cursor = 'grab'; e.preventDefault(); return; }
    if (e.key === 'p' || e.key === 'P') setMode('place');
    else if (e.key === 'c' || e.key === 'C') setMode('connect');
    else if (e.key === 'm' || e.key === 'M') setMode('move');
    else if (e.key === 'r' || e.key === 'R') rotateSelected();
    else if (e.key === 'k' || e.key === 'K') toggleCausalStroke();
    else if (e.key === 'Delete' || e.key === 'Backspace') deleteSelected();
    else if (e.key === 'Escape') {
        connectStartPort = null;
        clearPreview();
        clearPortHighlights();
        clearSelection();
        renderAll();
        updateStatus();
    }
});

document.addEventListener('keyup', (e) => {
    if (e.key === ' ') {
        spaceHeld = false;
        const cursors = { place: 'crosshair', connect: 'pointer', move: 'grab' };
        canvas.style.cursor = cursors[interactionMode] || 'default';
    }
});

// ---------------------------------------------------------------------------
// Toolbar buttons
// ---------------------------------------------------------------------------

document.getElementById('modePlace').addEventListener('click', () => setMode('place'));
document.getElementById('modeConnect').addEventListener('click', () => setMode('connect'));
document.getElementById('modeMove').addEventListener('click', () => setMode('move'));
document.getElementById('btnDelete').addEventListener('click', deleteSelected);
document.getElementById('btnClear').addEventListener('click', clearAll);
document.getElementById('btnExportTypst').addEventListener('click', copyTypstToClipboard);

const btnRotate = document.getElementById('btnRotate');
if (btnRotate) btnRotate.addEventListener('click', rotateSelected);

const btnCleanWires = document.getElementById('btnCleanWires');
if (btnCleanWires) btnCleanWires.addEventListener('click', autoRouteAllNets);

const btnVerify = document.getElementById('btnVerify');
if (btnVerify) btnVerify.addEventListener('click', verifyGraph);

document.getElementById('btnZoomIn').addEventListener('click', () => {
    const rect = canvasContainer.getBoundingClientRect();
    zoomAtPoint(rect.left + rect.width / 2, rect.top + rect.height / 2, viewState.zoom + ZOOM_STEP);
});
document.getElementById('btnZoomOut').addEventListener('click', () => {
    const rect = canvasContainer.getBoundingClientRect();
    zoomAtPoint(rect.left + rect.width / 2, rect.top + rect.height / 2, viewState.zoom - ZOOM_STEP);
});
document.getElementById('btnFit').addEventListener('click', fitToContent);

// ---------------------------------------------------------------------------
// Output panel
// ---------------------------------------------------------------------------

let activeTab = 'matrix';

document.querySelectorAll('.output-tab').forEach(tab => {
    tab.addEventListener('click', () => {
        document.querySelectorAll('.output-tab').forEach(t => t.classList.remove('active'));
        tab.classList.add('active');
        activeTab = tab.dataset.tab;
        updateOutput();
    });
});

function updateOutput() {
    try {
        if (activeTab === 'matrix') outputContent.innerHTML = renderMatrixHTML();
        else if (activeTab === 'ast') {
            const ast = buildEditorAST();
            outputContent.innerHTML = `<pre>${JSON.stringify(ast, null, 2)}</pre>`;
        }
        else if (activeTab === 'typst') outputContent.innerHTML = `<pre>${generateTypst()}</pre>`;
    } catch (e) {
        outputContent.innerHTML = `<pre style="color:#dc3545">Error: ${e.message}\n${e.stack || ''}</pre>`;
    }
}

// ---------------------------------------------------------------------------
// Incidence matrix generation
// ---------------------------------------------------------------------------

function buildIncidenceMatrix() {
    return buildIncidenceMatrixJS();
}

function buildPortIndex() {
    const ports = [];
    for (let ci = 0; ci < graphState.components.length; ci++) {
        const comp = graphState.components[ci];
        const def = COMPONENT_DEFS[comp.type];
        if (!def) continue;
        for (const pname of Object.keys(def.ports)) {
            ports.push({ ci, portName: pname });
        }
    }
    return ports;
}

function buildIncidenceMatrixJS() {
    const portIndex = buildPortIndex();
    const v = graphState.nets.length;
    const p = portIndex.length;
    const entries = [];

    for (let ni = 0; ni < graphState.nets.length; ni++) {
        const net = graphState.nets[ni];
        for (let ci_conn = 0; ci_conn < net.connections.length; ci_conn++) {
            const conn = net.connections[ci_conn];
            const ci = graphState.components.findIndex(c => c.id === conn.componentId);
            if (ci < 0) continue;
            const def = COMPONENT_DEFS[graphState.components[ci].type];
            if (!def) continue;
            const pi = portIndex.findIndex(e => e.ci === ci && e.portName === conn.portName);
            if (pi < 0) continue;
            const value = ci_conn === 0 ? 1 : -1;
            entries.push({ net: ni, port: pi, value });
        }
    }
    const port_labels = portIndex.map(e => `${e.ci}:${e.portName}`);
    return {
        entries, v, p, nnz: entries.length,
        net_labels: graphState.nets.map(n => n.label),
        port_labels,
    };
}

function cooToDense(inc) {
    const dense = Array.from({ length: inc.v }, () => Array(inc.p).fill(0));
    for (const e of inc.entries) dense[e.net][e.port] = e.value;
    return dense;
}

function renderMatrixHTML() {
    if (graphState.components.length === 0) {
        return '<pre style="color:#666">Place components and connect their ports to see the incidence matrix.</pre>';
    }

    const inc = buildIncidenceMatrix();
    const dense = cooToDense(inc);
    let html = '<table class="matrix-table"><tr><th></th>';
    for (const label of inc.port_labels) {
        const [ci, pname] = label.split(':');
        const comp = graphState.components[parseInt(ci, 10)];
        const shortType = comp ? (COMPONENT_DEFS[comp.type]?.label || comp.type) : ci;
        html += `<th>${shortType}<sub>${ci}</sub>:${pname}</th>`;
    }
    html += '</tr>';
    for (let i = 0; i < inc.v; i++) {
        html += `<tr><th>${inc.net_labels[i]}</th>`;
        for (let j = 0; j < inc.p; j++) {
            const val = dense[i][j];
            const cls = val > 0 ? 'pos' : val < 0 ? 'neg' : 'zero';
            html += `<td class="${cls}">${val}</td>`;
        }
        html += '</tr>';
    }
    html += '</table>';

    html += `<pre style="margin-top:12px;color:#888">V=${inc.v} nets, P=${inc.p} ports, nnz=${inc.nnz}</pre>`;
    return html;
}

// ---------------------------------------------------------------------------
// EditorNode AST generation
// ---------------------------------------------------------------------------

function buildEditorAST() {
    return buildEditorASTJS();
}

function buildEditorASTJS() {
    const inc = buildIncidenceMatrixJS();

    const flatTriples = [];
    for (const e of inc.entries) {
        flatTriples.push({ Const: String(e.net) });
        flatTriples.push({ Const: String(e.port) });
        flatTriples.push({ Const: String(e.value) });
    }
    const topologyNode = {
        Operation: {
            name: 'SparseMatrix',
            args: [
                { Const: String(inc.v) },
                { Const: String(inc.p) },
                { List: flatTriples }
            ]
        }
    };

    const componentNodes = graphState.components.map(comp => {
        const args = [];
        if (comp.params) {
            const def = COMPONENT_DEFS[comp.type];
            if (def && def.params) {
                for (const p of def.params) {
                    args.push({ Const: String(comp.params[p.name] ?? p.default) });
                }
            }
        }
        return { Operation: { name: comp.type, args } };
    });

    const netLabels = graphState.nets.map(net => ({ Const: `"${net.label}"` }));
    const portLabels = inc.port_labels.map(l => ({ Const: `"${l}"` }));

    return {
        Operation: {
            name: 'graph',
            args: [
                topologyNode,
                { List: componentNodes },
                { List: netLabels },
                { List: portLabels }
            ]
        }
    };
}

// ---------------------------------------------------------------------------
// Typst schematic export
// ---------------------------------------------------------------------------

// 1 canvas pixel = 0.02cm in Typst output
const PX_TO_CM = 0.02;

function pxToCm(px) {
    return (px * PX_TO_CM).toFixed(2);
}

function generateTypst() {
    if (graphState.components.length === 0) return '// Place components to generate Typst';

    // Compute bounding box to size the page
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const comp of graphState.components) {
        const { w, h } = getComponentSize(comp);
        minX = Math.min(minX, comp.x);
        minY = Math.min(minY, comp.y);
        maxX = Math.max(maxX, comp.x + w);
        maxY = Math.max(maxY, comp.y + h);
    }
    const margin = 40;
    const pageW = Math.max(8, Math.ceil((maxX - minX + margin * 2) * PX_TO_CM));
    const pageH = Math.max(6, Math.ceil((maxY - minY + margin * 2) * PX_TO_CM));
    const offX = minX - margin;
    const offY = minY - margin;

    let lines = [
        `#set page(width: ${pageW}cm, height: ${pageH}cm, margin: 0.5cm)`,
        '#set text(size: 8pt)',
        '',
    ];

    for (const comp of graphState.components) {
        const def = COMPONENT_DEFS[comp.type];
        const { w, h } = getComponentSize(comp);
        const dx = pxToCm(comp.x - offX);
        const dy = pxToCm(comp.y - offY);
        const tw = pxToCm(w);
        const th = pxToCm(h);
        const svgPath = (def.svg || '').replace(/^\//, '');
        const rot = comp.rotation || 0;

        if (rot !== 0) {
            lines.push(`#place(dx: ${dx}cm, dy: ${dy}cm, rotate(${rot}deg, image("${svgPath}", width: ${pxToCm(def.w)}cm, height: ${pxToCm(def.h)}cm)))`);
        } else {
            lines.push(`#place(dx: ${dx}cm, dy: ${dy}cm, image("${svgPath}", width: ${tw}cm, height: ${th}cm))`);
        }
    }

    lines.push('');

    function typstArrowhead(from, to) {
        const s = (v) => `${pxToCm(v)}cm`;
        const dx = to.x - from.x;
        const dy = to.y - from.y;
        const len = Math.hypot(dx, dy);
        if (len < 1) return null;
        const ux = dx / len, uy = dy / len;
        const headLen = 8;
        const headW = 4;
        const bx = to.x - ux * headLen, by = to.y - uy * headLen;
        const p1x = bx - uy * headW, p1y = by + ux * headW;
        const p2x = bx + uy * headW, p2y = by - ux * headW;
        return `#polygon(fill: black, (${s(to.x)}, ${s(to.y)}), (${s(p1x)}, ${s(p1y)}), (${s(p2x)}, ${s(p2y)}))`;
    }

    function typstCausalBar(from, to) {
        const s = (v) => `${pxToCm(v)}cm`;
        const dx = to.x - from.x;
        const dy = to.y - from.y;
        const len = Math.hypot(dx, dy);
        if (len < 1) return null;
        const ux = dx / len, uy = dy / len;
        const barW = 6;
        const p1x = to.x - uy * barW, p1y = to.y + ux * barW;
        const p2x = to.x + uy * barW, p2y = to.y - ux * barW;
        return `#line(start: (${s(p1x)}, ${s(p1y)}), end: (${s(p2x)}, ${s(p2y)}), stroke: 1pt)`;
    }

    function typstLines(pts, net) {
        const s = (v) => `${pxToCm(v)}cm`;
        const result = [];
        for (let i = 0; i < pts.length - 1; i++) {
            result.push(`#line(start: (${s(pts[i].x)}, ${s(pts[i].y)}), end: (${s(pts[i + 1].x)}, ${s(pts[i + 1].y)}), stroke: 0.5pt)`);
        }
        if (domainConfig.edge_decoration !== 'none' && pts.length >= 2) {
            const head = typstArrowhead(pts[pts.length - 2], pts[pts.length - 1]);
            if (head) result.push(head);
        }
        if (net && net.causal && pts.length >= 2) {
            if (net.causal === 'end') {
                const bar = typstCausalBar(pts[pts.length - 2], pts[pts.length - 1]);
                if (bar) result.push(bar);
            } else if (net.causal === 'start') {
                const bar = typstCausalBar(pts[1], pts[0]);
                if (bar) result.push(bar);
            }
        }
        return result;
    }

    for (const net of graphState.nets) {
        if (net.connections.length < 2) continue;

        if (net.connections.length === 2) {
            const pts = getNetPathPoints(net);
            const shifted = pts.map(p => ({ x: p.x - offX, y: p.y - offY }));
            lines.push(...typstLines(shifted, net));
        } else {
            const multiResult = getMultiNetLegs(net);
            if (multiResult && multiResult.legs) {
                for (const leg of multiResult.legs) {
                    const shifted = leg.map(p => ({ x: p.x - offX, y: p.y - offY }));
                    lines.push(...typstLines(shifted, net));
                }
            }
        }
    }

    return lines.join('\n');
}

function copyTypstToClipboard() {
    const typst = generateTypst();
    navigator.clipboard.writeText(typst).then(() => {
        statusBar.textContent = 'Typst copied to clipboard!';
        setTimeout(updateStatus, 2000);
    });
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

(async function initApp() {
    statusBar.textContent = 'Loading component definitions...';
    await loadComponentDefs();

    if (DOMAIN_FILTER) {
        const label = DOMAIN_FILTER.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
        const h1 = document.querySelector('header h1');
        if (h1) h1.textContent = `Kleis Graph Editor \u2014 ${label}`;
        document.title = `Kleis Graph Editor \u2014 ${label}`;
    }

    const types = Object.keys(COMPONENT_DEFS);
    if (types.length > 0) selectedPaletteType = types[0];

    buildPalette();
    applyViewBox();
    renderAll();
    updateOutput();
    updateStatus();

    window.addEventListener('resize', () => applyViewBox());
})();
