import { BVH as rt, HybridBuilder as at, WebGLCoordinateSystem as ct, WebGPUCoordinateSystem as ht, vec3ToArray as k, box3ToArray as H } from "./bvh.js";
import { Box3 as tt, Matrix4 as E, FloatType as ut, UnsignedIntType as lt, IntType as mt, DataTexture as ft, WebGLUtils as dt, ColorManagement as G, NoColorSpace as Y, RGBAFormat as gt, RGBAIntegerFormat as pt, RGFormat as xt, RGIntegerFormat as yt, RedFormat as _t, RedIntegerFormat as It, Frustum as wt, Vector3 as P, Sphere as et, Mesh as St, Ray as vt, BatchedMesh as d } from "./three.module.js";
import { radixSort as bt } from "./SortUtils.js";
class Ct {
  /**
   * @param target The target `BatchedMesh`.
   * @param margin The margin applied for bounding box calculations (default is 0).
   * @param accurateCulling Flag to enable accurate frustum culling without considering margin (default is true).
   */
  constructor(t, e, n = 0, s = !0) {
    this.nodesMap = /* @__PURE__ */ new Map(), this._origin = new Float32Array(3), this._dir = new Float32Array(3), this._cameraPos = new Float32Array(3), this._boxArray = new Float32Array(6), this.target = t, this.accurateCulling = s, this._margin = n, this.bvh = new rt(new at(), e === 2e3 ? ct : ht);
  }
  /**
   * Builds the BVH from the target mesh's instances using a top-down construction method.
   * This approach is more efficient and accurate compared to incremental methods, which add one instance at a time.
   */
  create() {
    const t = this.target.instanceCount, e = this.target._instanceInfo.length, n = this.target._instanceInfo, s = new Array(t), o = new Uint32Array(t);
    let r = 0;
    this.clear();
    for (let a = 0; a < e; a++)
      n[a].active && (s[r] = this.getBox(a, new Float32Array(6)), o[r] = a, r++);
    this.bvh.createFromArray(o, s, (a) => {
      this.nodesMap.set(a.object, a);
    }, this._margin);
  }
  /**
   * Inserts an instance into the BVH.
   * @param id The id of the instance to insert.
   */
  insert(t) {
    const e = this.bvh.insert(t, this.getBox(t, new Float32Array(6)), this._margin);
    this.nodesMap.set(t, e);
  }
  /**
   * Inserts a range of instances into the BVH.
   * @param ids An array of ids to insert.
   */
  insertRange(t) {
    const e = t.length, n = new Array(e);
    for (let s = 0; s < e; s++)
      n[s] = this.getBox(t[s], new Float32Array(6));
    this.bvh.insertRange(t, n, this._margin, (s) => {
      this.nodesMap.set(s.object, s);
    });
  }
  /**
   * Moves an instance within the BVH.
   * @param id The id of the instance to move.
   */
  move(t) {
    const e = this.nodesMap.get(t);
    e && (this.getBox(t, e.box), this.bvh.move(e, this._margin));
  }
  /**
   * Deletes an instance from the BVH.
   * @param id The id of the instance to delete.
   */
  delete(t) {
    const e = this.nodesMap.get(t);
    e && (this.bvh.delete(e), this.nodesMap.delete(t));
  }
  /**
   * Clears the BVH.
   */
  clear() {
    this.bvh.clear(), this.nodesMap.clear();
  }
  /**
   * Performs frustum culling to determine which instances are visible based on the provided projection matrix.
   * @param projScreenMatrix The projection screen matrix for frustum culling.
   * @param onFrustumIntersection Callback function invoked when an instance intersects the frustum.
   */
  frustumCulling(t, e) {
    this._margin > 0 && this.accurateCulling ? this.bvh.frustumCulling(t.elements, (n, s, o) => {
      s.isIntersectedMargin(n.box, o, this._margin) && e(n);
    }) : this.bvh.frustumCulling(t.elements, e);
  }
  /**
   * Performs raycasting to check if a ray intersects any instances.
   * @param raycaster The raycaster used for raycasting.
   * @param onIntersection Callback function invoked when a ray intersects an instance.
   */
  raycast(t, e) {
    const n = t.ray, s = this._origin, o = this._dir;
    k(n.origin, s), k(n.direction, o), this.bvh.rayIntersections(o, s, e, t.near, t.far);
  }
  /**
   * Checks if a given box intersects with any instance bounding box.
   * @param target The target bounding box.
   * @param onIntersection Callback function invoked when an intersection occurs.
   * @returns `True` if there is an intersection, otherwise `false`.
   */
  intersectBox(t, e) {
    const n = this._boxArray;
    return H(t, n), this.bvh.intersectsBox(n, e);
  }
  getBox(t, e) {
    const n = this.target, s = n._instanceInfo[t].geometryIndex;
    return n.getBoundingBoxAt(s, N).applyMatrix4(n.getMatrixAt(t, Mt)), H(N, e), e;
  }
}
const N = new tt(), Mt = new E();
class Ut {
  constructor() {
    this.array = [], this.pool = [];
  }
  push(t, e, n, s) {
    const o = this.pool, r = this.array, a = r.length;
    a >= o.length && o.push({ start: null, count: null, z: null, zSort: null, index: null });
    const c = o[a];
    c.index = t, c.start = n, c.count = s, c.z = e, r.push(c);
  }
  reset() {
    this.array.length = 0;
  }
}
function nt(i, t) {
  return Math.max(t, Math.ceil(Math.sqrt(i / t)) * t);
}
function At(i, t, e, n) {
  t === 3 && (console.warn('"channels" cannot be 3. Set to 4. More info: https://github.com/mrdoob/three.js/pull/23228'), t = 4);
  const s = nt(n, e), o = new i(s * s * t), r = i.name.includes("Float"), a = i.name.includes("Uint"), c = r ? ut : a ? lt : mt;
  let m;
  switch (t) {
    case 1:
      m = r ? _t : It;
      break;
    case 2:
      m = r ? xt : yt;
      break;
    case 4:
      m = r ? gt : pt;
      break;
  }
  return { array: o, size: s, type: c, format: m };
}
class Lt extends ft {
  /**
   * @param arrayType The constructor for the TypedArray.
   * @param channels The number of channels in the texture.
   * @param pixelsPerInstance The number of pixels required for each instance.
   * @param capacity The total number of instances.
   * @param uniformMap Optional map for handling uniform values.
   * @param fetchInFragmentShader Optional flag that determines if uniform values should be fetched in the fragment shader instead of the vertex shader.
   */
  constructor(t, e, n, s, o, r) {
    e === 3 && (e = 4);
    const { array: a, format: c, size: m, type: h } = At(t, e, n, s);
    super(a, m, m, c, h), this.partialUpdate = !0, this.maxUpdateCalls = 1 / 0, this._utils = null, this._needsUpdate = !1, this._lastWidth = null, this._data = a, this._channels = e, this._pixelsPerInstance = n, this._stride = n * e, this._rowToUpdate = new Array(m), this._uniformMap = o, this._fetchUniformsInFragmentShader = r, this.needsUpdate = !0;
  }
  /**
   * Resizes the texture to accommodate a new number of instances.
   * @param count The new total number of instances.
   */
  resize(t) {
    const e = nt(t, this._pixelsPerInstance);
    if (e === this.image.width) return;
    const n = this._data, s = this._channels;
    this._rowToUpdate.length = e;
    const o = n.constructor, r = new o(e * e * s), a = Math.min(n.length, r.length);
    r.set(new o(n.buffer, 0, a)), this.dispose(), this.image = { data: r, height: e, width: e }, this._data = r;
  }
  /**
   * Marks a row of the texture for update during the next render cycle.
   * This helps in optimizing texture updates by only modifying the rows that have changed.
   * @param index The index of the instance to update.
   */
  enqueueUpdate(t) {
    if (this._needsUpdate = !0, !this.partialUpdate) return;
    const e = this.image.width / this._pixelsPerInstance, n = Math.floor(t / e);
    this._rowToUpdate[n] = !0;
  }
  /**
   * Updates the texture data based on the rows that need updating.
   * This method is optimized to only update the rows that have changed, improving performance.
   * @param renderer The WebGLRenderer used for rendering.
   */
  update(t) {
    const e = t.properties.get(this), n = this.version > 0 && e.__version !== this.version, s = this._lastWidth !== null && this._lastWidth !== this.image.width;
    if (!this._needsUpdate || !e.__webglTexture || n || s) {
      this._lastWidth = this.image.width, this._needsUpdate = !1;
      return;
    }
    if (this._needsUpdate = !1, !this.partialUpdate) {
      this.needsUpdate = !0;
      return;
    }
    const o = this.getUpdateRowsInfo();
    o.length !== 0 && (o.length > this.maxUpdateCalls ? this.needsUpdate = !0 : this.updateRows(e, t, o), this._rowToUpdate.fill(!1));
  }
  // TODO reuse same objects to prevent memory leak
  getUpdateRowsInfo() {
    const t = this._rowToUpdate, e = [];
    for (let n = 0, s = t.length; n < s; n++)
      if (t[n]) {
        const o = n;
        for (; n < s && t[n]; n++)
          ;
        e.push({ row: o, count: n - o });
      }
    return e;
  }
  updateRows(t, e, n) {
    const s = e.state, o = e.getContext();
    this._utils ?? (this._utils = new dt(o, e.extensions, e.capabilities));
    const r = this._utils.convert(this.format), a = this._utils.convert(this.type), { data: c, width: m } = this.image, h = this._channels;
    s.bindTexture(o.TEXTURE_2D, t.__webglTexture);
    const u = G.getPrimaries(G.workingColorSpace), l = this.colorSpace === Y ? null : G.getPrimaries(this.colorSpace), f = this.colorSpace === Y || u === l ? o.NONE : o.BROWSER_DEFAULT_WEBGL;
    o.pixelStorei(o.UNPACK_FLIP_Y_WEBGL, this.flipY), o.pixelStorei(o.UNPACK_PREMULTIPLY_ALPHA_WEBGL, this.premultiplyAlpha), o.pixelStorei(o.UNPACK_ALIGNMENT, this.unpackAlignment), o.pixelStorei(o.UNPACK_COLORSPACE_CONVERSION_WEBGL, f);
    for (const { count: g, row: w } of n)
      o.texSubImage2D(o.TEXTURE_2D, 0, 0, w, m, g, r, a, c, w * m * h);
    this.onUpdate && this.onUpdate(this);
  }
  /**
   * Sets a uniform value at the specified instance ID in the texture.
   * @param id The instance ID to set the uniform for.
   * @param name The name of the uniform.
   * @param value The value to set for the uniform.
   */
  setUniformAt(t, e, n) {
    const { offset: s, size: o } = this._uniformMap.get(e), r = this._stride;
    o === 1 ? this._data[t * r + s] = n : n.toArray(this._data, t * r + s);
  }
  /**
   * Retrieves a uniform value at the specified instance ID from the texture.
   * @param id The instance ID to retrieve the uniform from.
   * @param name The name of the uniform.
   * @param target Optional target object to store the uniform value.
   * @returns The uniform value for the specified instance.
   */
  getUniformAt(t, e, n) {
    const { offset: s, size: o } = this._uniformMap.get(e), r = this._stride;
    return o === 1 ? this._data[t * r + s] : n.fromArray(this._data, t * r + s);
  }
  /**
   * Generates the GLSL code for accessing the uniform data stored in the texture.
   * @param textureName The name of the texture in the GLSL shader.
   * @param indexName The name of the index in the GLSL shader.
   * @param indexType The type of the index in the GLSL shader.
   * @returns An object containing the GLSL code for the vertex and fragment shaders.
   */
  getUniformsGLSL(t, e, n) {
    const s = this.getUniformsVertexGLSL(t, e, n), o = this.getUniformsFragmentGLSL(t, e, n);
    return { vertex: s, fragment: o };
  }
  getUniformsVertexGLSL(t, e, n) {
    if (this._fetchUniformsInFragmentShader)
      return `
        flat varying ${n} ez_v${e}; 
        void main() {
          ez_v${e} = ${e};`;
    const s = this.texelsFetchGLSL(t, e), o = this.getFromTexelsGLSL(), { assignVarying: r, declareVarying: a } = this.getVarying();
    return `
      uniform highp sampler2D ${t};  
      ${a}
      void main() {
        ${s}
        ${o}
        ${r}`;
  }
  getUniformsFragmentGLSL(t, e, n) {
    if (!this._fetchUniformsInFragmentShader) {
      const { declareVarying: r, getVarying: a } = this.getVarying();
      return `
      ${r}
      void main() {
        ${a}`;
    }
    const s = this.texelsFetchGLSL(t, `ez_v${e}`), o = this.getFromTexelsGLSL();
    return `
      uniform highp sampler2D ${t};  
      flat varying ${n} ez_v${e};
      void main() {
        ${s}
        ${o}`;
  }
  texelsFetchGLSL(t, e) {
    const n = this._pixelsPerInstance;
    let s = `
      int size = textureSize(${t}, 0).x;
      int j = int(${e}) * ${n};
      int x = j % size;
      int y = j / size;
    `;
    for (let o = 0; o < n; o++)
      s += `vec4 ez_texel${o} = texelFetch(${t}, ivec2(x + ${o}, y), 0);
`;
    return s;
  }
  getFromTexelsGLSL() {
    const t = this._uniformMap;
    let e = "";
    for (const [n, { type: s, offset: o, size: r }] of t) {
      const a = Math.floor(o / this._channels);
      if (s === "mat3")
        e += `mat3 ${n} = mat3(ez_texel${a}.rgb, vec3(ez_texel${a}.a, ez_texel${a + 1}.rg), vec3(ez_texel${a + 1}.ba, ez_texel${a + 2}.r));
`;
      else if (s === "mat4")
        e += `mat4 ${n} = mat4(ez_texel${a}, ez_texel${a + 1}, ez_texel${a + 2}, ez_texel${a + 3});
`;
      else {
        const c = this.getUniformComponents(o, r);
        e += `${s} ${n} = ez_texel${a}.${c};
`;
      }
    }
    return e;
  }
  getVarying() {
    const t = this._uniformMap;
    let e = "", n = "", s = "";
    for (const [o, { type: r }] of t)
      e += `flat varying ${r} ez_v${o};
`, n += `ez_v${o} = ${o};
`, s += `${r} ${o} = ez_v${o};
`;
    return { declareVarying: e, assignVarying: n, getVarying: s };
  }
  getUniformComponents(t, e) {
    const n = t % this._channels;
    let s = "";
    for (let o = 0; o < e; o++)
      s += Tt[n + o];
    return s;
  }
  copy(t) {
    return super.copy(t), this.partialUpdate = t.partialUpdate, this.maxUpdateCalls = t.maxUpdateCalls, this._channels = t._channels, this._pixelsPerInstance = t._pixelsPerInstance, this._stride = t._stride, this._rowToUpdate = t._rowToUpdate, this._uniformMap = t._uniformMap, this._fetchUniformsInFragmentShader = t._fetchUniformsInFragmentShader, this;
  }
}
const Tt = ["r", "g", "b", "a"];
function Dt(i, t = {}) {
  this.bvh = new Ct(this, i, t.margin, t.accurateCulling), this.bvh.create();
}
function re(i) {
  const t = {
    get: (e) => e.zSort,
    aux: new Array(i.maxInstanceCount),
    reversed: null
  };
  return function(n) {
    t.reversed = i.material.transparent, i.maxInstanceCount > t.aux.length && (t.aux.length = i.maxInstanceCount);
    let s = 1 / 0, o = -1 / 0;
    for (const { z: c } of n)
      c > o && (o = c), c < s && (s = c);
    const r = o - s, a = (2 ** 32 - 1) / r;
    for (const c of n)
      c.zSort = (c.z - s) * a;
    bt(n, t);
  };
}
function Ft(i, t) {
  return i.z - t.z;
}
function Pt(i, t) {
  return t.z - i.z;
}
const K = new wt(), M = new Ut(), W = new E(), B = new E(), $ = new P(), O = new P(), V = new P(), zt = new P(), b = new et();
function Bt(i, t, e, n, s, o) {
  var r;
  this.frustumCulling(e), (r = this.uniformsTexture) == null || r.update(i);
}
function Et(i, t = i) {
  if (!this._visibilityChanged && !this.perObjectFrustumCulled && !this.sortObjects)
    return;
  this._indirectTexture.needsUpdate = !0, this._visibilityChanged = !1;
  const e = this.sortObjects, n = this.perObjectFrustumCulled;
  if (!n && !e) {
    this.updateIndexArray();
    return;
  }
  if (B.copy(this.matrixWorld).invert(), O.setFromMatrixPosition(i.matrixWorld).applyMatrix4(B), V.setFromMatrixPosition(t.matrixWorld).applyMatrix4(B), $.set(0, 0, -1).transformDirection(i.matrixWorld).transformDirection(B), n ? (W.multiplyMatrices(i.projectionMatrix, i.matrixWorldInverse).multiply(this.matrixWorld), this.bvh ? this.BVHCulling(i, t) : this.linearCulling(i, t)) : this.updateRenderList(), e) {
    const s = this.geometry.getIndex(), o = s === null ? 1 : s.array.BYTES_PER_ELEMENT, r = this._multiDrawStarts, a = this._multiDrawCounts, c = this._indirectTexture.image.data, m = this.customSort;
    m === null ? M.array.sort(this.material.transparent ? Pt : Ft) : m(M.array);
    const h = M.array, u = h.length;
    for (let l = 0; l < u; l++) {
      const f = h[l];
      r[l] = f.start * o, a[l] = f.count, c[l] = f.index;
    }
    M.reset();
  }
}
function $t() {
  if (!this._visibilityChanged) return;
  const i = this.geometry.getIndex(), t = i === null ? 1 : i.array.BYTES_PER_ELEMENT, e = this._instanceInfo, n = this._geometryInfo, s = this._multiDrawStarts, o = this._multiDrawCounts, r = this._indirectTexture.image.data;
  let a = 0;
  for (let c = 0, m = e.length; c < m; c++) {
    const h = e[c];
    if (h.visible && h.active) {
      const u = h.geometryIndex, l = n[u];
      s[a] = l.start * t, o[a] = l.count, r[a] = c, a++;
    }
  }
  this._multiDrawCount = a;
}
function Ot() {
  const i = this._instanceInfo, t = this._geometryInfo;
  for (let e = 0, n = i.length; e < n; e++) {
    const s = i[e];
    if (s.visible && s.active) {
      const o = s.geometryIndex, r = t[o], a = this.getPositionAt(e).sub(O).dot($);
      M.push(e, a, r.start, r.count);
    }
  }
  this._multiDrawCount = M.array.length;
}
function Rt(i, t) {
  const e = this.geometry.getIndex(), n = e === null ? 1 : e.array.BYTES_PER_ELEMENT, s = this._instanceInfo, o = this._geometryInfo, r = this.sortObjects, a = this._multiDrawStarts, c = this._multiDrawCounts, m = this._indirectTexture.image.data, h = this.onFrustumEnter;
  let u = 0;
  const l = i, f = (l.top - l.bottom) / l.zoom, g = i, w = Math.tan(g.fov * 0.5 * (Math.PI / 180)) ** 2, U = this.useDistanceForLOD, D = g.isPerspectiveCamera;
  this.bvh.frustumCulling(W, (_) => {
    const v = _.object, A = s[v];
    if (!A.visible) return;
    const z = A.geometryIndex, I = o[z], p = I.LOD;
    let x, y;
    if (p) {
      b.radius = I.boundingSphere.radius;
      let S;
      if (D) {
        const T = this.getPositionAt(v).distanceToSquared(V);
        S = st(U, b, w, T);
      } else
        S = ot(U, b, f);
      const L = this.getLODIndex(p, S, D);
      if (h && !h(v, i, t, L)) return;
      x = p[L].start, y = p[L].count;
    } else {
      if (h && !h(v, i)) return;
      x = I.start, y = I.count;
    }
    if (r) {
      const S = this.getPositionAt(v).sub(O).dot($);
      M.push(v, S, x, y);
    } else
      a[u] = x * n, c[u] = y, m[u] = v, u++;
  }), this._multiDrawCount = r ? M.array.length : u;
}
function st(i, t, e, n) {
  return i ? n : t.radius ** 2 / (n * e);
}
function ot(i, t, e) {
  if (i) throw new Error("BatchedMesh: useDistanceForLOD cannot be used with orthographic camera.");
  return t.radius * 2 / e;
}
function Gt(i, t) {
  const e = this.geometry.getIndex(), n = e === null ? 1 : e.array.BYTES_PER_ELEMENT, s = this._instanceInfo, o = this._geometryInfo, r = this.sortObjects, a = this._multiDrawStarts, c = this._multiDrawCounts, m = this._indirectTexture.image.data, h = this.onFrustumEnter;
  let u = 0;
  K.setFromProjectionMatrix(W);
  const l = i, f = (l.top - l.bottom) / l.zoom, g = i, w = Math.tan(g.fov * 0.5 * (Math.PI / 180)) ** 2, U = this.useDistanceForLOD, D = g.isPerspectiveCamera;
  for (let _ = 0, v = s.length; _ < v; _++) {
    const A = s[_];
    if (!A.visible || !A.active) continue;
    const z = A.geometryIndex, I = o[z], p = I.LOD;
    let x, y;
    const S = I.boundingSphere, L = S.radius, T = S.center;
    if (T.x === 0 && T.y === 0 && T.z === 0) {
      const F = this.getPositionAndMaxScaleOnAxisAt(_, b.center);
      b.radius = L * F;
    } else
      this.applyMatrixAtToSphere(_, b, T, L);
    if (K.intersectsSphere(b)) {
      if (p) {
        let F;
        if (D) {
          const it = b.center.distanceToSquared(V);
          F = st(U, b, w, it);
        } else
          F = ot(U, b, f);
        const R = this.getLODIndex(p, F, D);
        if (h && !h(_, i, t, R)) continue;
        x = p[R].start, y = p[R].count;
      } else {
        if (h && !h(_, i)) continue;
        x = I.start, y = I.count;
      }
      if (r) {
        const F = zt.subVectors(b.center, O).dot($);
        M.push(_, F, x, y);
      } else
        a[u] = x * n, c[u] = y, m[u] = _, u++;
    }
  }
  this._multiDrawCount = r ? M.array.length : u;
}
const jt = new P();
function Wt(i, t = jt) {
  const e = i * 16, n = this._matricesTexture.image.data;
  return t.x = n[e + 12], t.y = n[e + 13], t.z = n[e + 14], t;
}
function Vt(i, t) {
  const e = i * 16, n = this._matricesTexture.image.data, s = n[e + 0], o = n[e + 1], r = n[e + 2], a = s * s + o * o + r * r, c = n[e + 4], m = n[e + 5], h = n[e + 6], u = c * c + m * m + h * h, l = n[e + 8], f = n[e + 9], g = n[e + 10], w = l * l + f * f + g * g;
  return t.x = n[e + 12], t.y = n[e + 13], t.z = n[e + 14], Math.sqrt(Math.max(a, u, w));
}
function qt(i, t, e, n) {
  const s = i * 16, o = this._matricesTexture.image.data, r = o[s + 0], a = o[s + 1], c = o[s + 2], m = o[s + 3], h = o[s + 4], u = o[s + 5], l = o[s + 6], f = o[s + 7], g = o[s + 8], w = o[s + 9], U = o[s + 10], D = o[s + 11], _ = o[s + 12], v = o[s + 13], A = o[s + 14], z = o[s + 15], I = t.center, p = e.x, x = e.y, y = e.z, S = 1 / (m * p + f * x + D * y + z);
  I.x = (r * p + h * x + g * y + _) * S, I.y = (a * p + u * x + w * y + v) * S, I.z = (c * p + l * x + U * y + A) * S;
  const L = r * r + a * a + c * c, T = h * h + u * u + l * l, q = g * g + w * w + U * U;
  t.radius = n * Math.sqrt(Math.max(L, T, q));
}
function kt(i, t, e) {
  const n = this._geometryInfo[i], s = t.isBufferGeometry ? t.index.array : t, o = e ** 2;
  n.LOD ?? (n.LOD = [{ start: n.start, count: n.count, metric: 1 / 0, metricSquared: 1 / 0 }]);
  const r = n.LOD, a = r[r.length - 1], c = a.start + a.count, m = s.length;
  if (c - n.start + m > n.reservedIndexCount)
    throw new Error("BatchedMesh LOD: Reserved space request exceeds the maximum buffer size.");
  r.push({ start: c, count: m, metric: e, metricSquared: o });
  const h = this.geometry.getIndex(), u = h.array, l = n.vertexStart;
  for (let f = 0; f < m; f++)
    u[c + f] = s[f] + l;
  h.needsUpdate = !0;
}
function Ht(i, t, e = !1) {
  const n = e ? "metricSquared" : "metric";
  if (this.useDistanceForLOD) {
    for (let s = i.length - 1; s > 0; s--) {
      const r = i[s][n];
      if (t >= r) return s;
    }
    return 0;
  }
  for (let s = i.length - 1; s > 0; s--) {
    const r = i[s][n];
    if (t <= r) return s;
  }
  return 0;
}
const j = [], C = new St(), Yt = new vt(), X = new P(), Z = new P(), J = new E();
function Nt(i, t) {
  var r, a;
  if (!this.material || this.instanceCount === 0) return;
  C.geometry = this.geometry, C.material = this.material, (r = C.geometry).boundingBox ?? (r.boundingBox = new tt()), (a = C.geometry).boundingSphere ?? (a.boundingSphere = new et());
  const e = i.ray, n = i.near, s = i.far;
  J.copy(this.matrixWorld).invert(), Z.setFromMatrixScale(this.matrixWorld), X.copy(i.ray.direction).multiply(Z);
  const o = X.length();
  if (i.ray = Yt.copy(i.ray).applyMatrix4(J), i.near /= o, i.far /= o, this.bvh)
    this.bvh.raycast(i, (c) => this.checkInstanceIntersection(i, c, t));
  else if (this.boundingSphere === null && this.computeBoundingSphere(), i.ray.intersectsSphere(this.boundingSphere))
    for (let c = 0, m = this._instanceInfo.length; c < m; c++)
      this.checkInstanceIntersection(i, c, t);
  i.ray = e, i.near = n, i.far = s;
}
function Kt(i, t, e) {
  const n = this._instanceInfo[t];
  if (!n.active || !n.visible) return;
  const s = n.geometryIndex, o = this._geometryInfo[s];
  this.getMatrixAt(t, C.matrixWorld), C.geometry.boundsTree = this.boundsTrees ? this.boundsTrees[s] : void 0, C.geometry.boundsTree || (this.getBoundingBoxAt(s, C.geometry.boundingBox), this.getBoundingSphereAt(s, C.geometry.boundingSphere), C.geometry.setDrawRange(o.start, o.count)), C.raycast(i, j);
  for (const r of j)
    r.batchId = t, r.object = this, e.push(r);
  j.length = 0;
}
function Xt(i) {
  const t = i.material, e = t.onBeforeCompile.bind(t);
  t.onBeforeCompile = (n, s) => {
    if (i.uniformsTexture) {
      n.uniforms.uniformsTexture = { value: i.uniformsTexture };
      const { vertex: o, fragment: r } = i.uniformsTexture.getUniformsGLSL("uniformsTexture", "batchIndex", "float");
      n.vertexShader = n.vertexShader.replace("void main() {", o), n.fragmentShader = n.fragmentShader.replace("void main() {", r), n.vertexShader = n.vertexShader.replace("void main() {", "void main() { float batchIndex = getIndirectIndex( gl_DrawID );");
    }
    e(n, s);
  };
}
function Zt(i, t, e) {
  if (!this.uniformsTexture)
    throw new Error(`Before get/set uniform, it's necessary to use "initUniformsPerInstance".`);
  return this.uniformsTexture.getUniformAt(i, t, e);
}
function Jt(i, t, e) {
  if (!this.uniformsTexture)
    throw new Error(`Before get/set uniform, it's necessary to use "initUniformsPerInstance".`);
  this.uniformsTexture.setUniformAt(i, t, e), this.uniformsTexture.enqueueUpdate(i);
}
function Qt(i) {
  if (this.uniformsTexture) throw new Error('"initUniformsPerInstance" must be called only once.');
  const { channels: t, pixelsPerInstance: e, uniformMap: n, fetchInFragmentShader: s } = te(i);
  this.uniformsTexture = new Lt(Float32Array, t, e, this.maxInstanceCount, n, s), Xt(this);
}
function te(i) {
  let t = 0;
  const e = /* @__PURE__ */ new Map(), n = [], s = i.vertex ?? {}, o = i.fragment ?? {};
  let r = !0;
  for (const h in s) {
    const u = s[h], l = Q(u);
    t += l, n.push({ name: h, type: u, size: l }), r = !1;
  }
  for (const h in o)
    if (!s[h]) {
      const u = o[h], l = Q(u);
      t += l, n.push({ name: h, type: u, size: l });
    }
  n.sort((h, u) => u.size - h.size);
  const a = [];
  for (const { name: h, size: u, type: l } of n) {
    const f = ee(u, a);
    e.set(h, { offset: f, size: u, type: l });
  }
  const c = Math.ceil(t / 4);
  return { channels: Math.min(t, 4), pixelsPerInstance: c, uniformMap: e, fetchInFragmentShader: r };
}
function ee(i, t) {
  if (i < 4) {
    for (let n = 0; n < t.length; n++)
      if (t[n] + i <= 4) {
        const s = n * 4 + t[n];
        return t[n] += i, s;
      }
  }
  const e = t.length * 4;
  for (; i > 0; i -= 4)
    t.push(i);
  return e;
}
function Q(i) {
  switch (i) {
    case "float":
      return 1;
    case "vec2":
      return 2;
    case "vec3":
      return 3;
    case "vec4":
      return 4;
    case "mat3":
      return 9;
    case "mat4":
      return 16;
    default:
      throw new Error(`Invalid uniform type: ${i}`);
  }
}
function ne() {
  d.prototype.computeBVH = Dt, d.prototype.onBeforeRender = Bt, d.prototype.frustumCulling = Et, d.prototype.updateIndexArray = $t, d.prototype.updateRenderList = Ot, d.prototype.BVHCulling = Rt, d.prototype.linearCulling = Gt, d.prototype.getPositionAt = Wt, d.prototype.getPositionAndMaxScaleOnAxisAt = Vt, d.prototype.applyMatrixAtToSphere = qt, d.prototype.addGeometryLOD = kt, d.prototype.getLODIndex = Ht, d.prototype.raycast = Nt, d.prototype.checkInstanceIntersection = Kt;
}
function ae() {
  ne(), d.prototype.getUniformAt = Zt, d.prototype.setUniformAt = Jt, d.prototype.initUniformsPerInstance = Qt;
}
function ce(i) {
  let t = 0, e = 0;
  for (const n of i)
    t += n.attributes.position.count, e += n.index.count;
  return { vertexCount: t, indexCount: e };
}
function he(i) {
  const t = [];
  let e = 0, n = 0;
  for (const s of i) {
    let o = 0;
    for (const r of s) {
      const a = r.index.count;
      n += a, o += a, e += r.attributes.position.count;
    }
    t.push(o);
  }
  return { vertexCount: e, indexCount: n, LODIndexCount: t };
}
export {
  Rt as BVHCulling,
  Ct as BatchedMeshBVH,
  Ut as MultiDrawRenderList,
  Lt as SquareDataTexture,
  kt as addGeometryLOD,
  qt as applyMatrixAtToSphere,
  Kt as checkInstanceIntersection,
  Dt as computeBVH,
  re as createRadixSort,
  ae as extendBatchedMeshPrototype,
  Et as frustumCulling,
  ce as getBatchedMeshCount,
  he as getBatchedMeshLODCount,
  Ht as getLODIndex,
  Vt as getPositionAndMaxScaleOnAxisAt,
  Wt as getPositionAt,
  At as getSquareTextureInfo,
  nt as getSquareTextureSize,
  Zt as getUniformAt,
  ee as getUniformOffset,
  te as getUniformSchemaResult,
  Q as getUniformSize,
  Qt as initUniformsPerInstance,
  Gt as linearCulling,
  Bt as onBeforeRender,
  Xt as patchBatchedMeshMaterial,
  Nt as raycast,
  Jt as setUniformAt,
  Ft as sortOpaque,
  Pt as sortTransparent,
  $t as updateIndexArray,
  Ot as updateRenderList
};
//# sourceMappingURL=webgl.js.map
