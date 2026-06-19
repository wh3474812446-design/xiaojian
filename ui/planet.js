/* 钴蓝粒子星球 —— 移植自 Eamon 作品集入场券页面(回归原作配色),嵌入左下角「正在监听」卡片。
   纯 2D Canvas,无外部依赖、不联网;随监听状态联动(暂停时降速并去饱和);
   配色为钴蓝小粒子(粒子小、分布散、无发光),浮于卡片淡蓝票据块上,呼应 Eamon 入场券;
   点击星球可把附近粒子炸散,随后由弹簧力自动聚拢(还原作品集原版交互)。 */
(function () {
  const canvas = document.getElementById('watch-planet');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  const banner = document.getElementById('watch-banner');
  const dpr = Math.min(window.devicePixelRatio || 1, 2);

  let width = 0, height = 0, cx = 0, cy = 0, planetRadius = 0, raf = 0;
  let particles = [];
  let built = false;

  let paused = banner ? banner.classList.contains('paused') : false;
  let speedFactor = paused ? 0.12 : 1;
  let desat = paused ? 1 : 0;
  let spin = 0;
  let breath = 0;

  const BODY_COUNT = 580;   // 中间球体粒子数
  const RING_COUNT = 240;
  const RING_INNER = 1.22;
  const RING_OUTER = 1.90;
  const RING_TILT = 0.40;
  const GRAY = [150, 152, 162];   // 暂停时去饱和目标：蓝灰(浅底上褪为静止灰球)

  const seededRandom = (n) => {
    const seed = Math.sin(n * 9301.7 + 49297.3) * 233280;
    return seed - Math.floor(seed);
  };
  const lerp = (a, b, t) => a + (b - a) * t;

  const build = () => {
    particles = [];
    for (let i = 0; i < BODY_COUNT; i++) {
      const u = seededRandom(i * 7 + 3) * 2 - 1;
      const phi = seededRandom(i * 11 + 5) * Math.PI * 2;
      const sq = Math.sqrt(Math.max(0, 1 - u * u));
      const dx = sq * Math.cos(phi);
      const dy = u;
      const dz = sq * Math.sin(phi);
      const rr = Math.pow(seededRandom(i * 17 + 9), 0.40);
      particles.push({
        hx: dx * rr, hy: dy * rr, hz: dz * rr,
        x: dx * rr, y: dy * rr, z: dz * rr,
        vx: 0, vy: 0, vz: 0,
        type: 'body',
        size: 0.5 + seededRandom(i * 11 + 5) * 0.45,
        colorT: seededRandom(i * 13 + 7),
      });
    }
    for (let i = 0; i < RING_COUNT; i++) {
      const angle = (i / RING_COUNT) * Math.PI * 2 + seededRandom(i * 17 + 3) * 0.6;
      const rr = RING_INNER + seededRandom(i * 19 + 9) * (RING_OUTER - RING_INNER);
      const x0 = rr * Math.cos(angle);
      const y0 = -rr * Math.sin(angle) * Math.sin(RING_TILT);
      const z0 = rr * Math.sin(angle) * Math.cos(RING_TILT);
      particles.push({
        hx: x0, hy: y0, hz: z0,
        x: x0, y: y0, z: z0,
        vx: 0, vy: 0, vz: 0,
        type: 'ring',
        size: 0.3 + seededRandom(i * 23 + 7) * 0.4,
        colorT: seededRandom(i * 29 + 11),
      });
    }
    built = true;
  };

  const resize = () => {
    width = canvas.clientWidth;
    height = canvas.clientHeight;
    if (width < 2 || height < 2) return false;
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.scale(dpr, dpr);
    cx = width * 0.5;
    cy = height * 0.5;
    planetRadius = Math.min(width * 0.30, height * 0.34, 34);
    return true;
  };

  // 点击散开:对鼠标半径内的粒子施加 径向向外 + 切向旋涡 的冲量(还原作品集原版),
  // 随后 draw() 里的弹簧力会把它们自动拉回原位。冲量系数已按小卡片尺寸放大。
  const scatter = (mx, my) => {
    const cosY = Math.cos(spin);
    const sinY = Math.sin(spin);
    const breathe = 1 + 0.05 * Math.sin(breath);
    const radius = Math.max(72, planetRadius * 2.2);
    for (const p of particles) {
      const rx = p.x * cosY + p.z * sinY;
      const rz = -p.x * sinY + p.z * cosY;
      const ry = p.y * breathe;
      const fov = 3.8;
      const scale = fov / (fov + rz * 0.35);
      const sx = cx + rx * scale * planetRadius;
      const sy = cy + ry * scale * planetRadius;
      const dx = sx - mx;
      const dy = sy - my;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < radius) {
        const force = Math.pow(1 - dist / radius, 1.4) * 0.7;
        const angle = Math.atan2(dy, dx);
        const tx = -Math.sin(angle);
        const ty = Math.cos(angle);
        // 冲量已调小,散开速度更慢更柔(配合 draw() 里更低的回拉力 + 更高阻尼)
        p.vx += (Math.cos(angle) * 0.65 + tx * 0.35) * force * 0.13;
        p.vy += (Math.sin(angle) * 0.65 + ty * 0.35) * force * 0.13;
        p.vz += (Math.random() - 0.5) * force * 0.08;
      }
    }
  };

  const draw = () => {
    if (!width || !height) {
      if (!resize()) { raf = requestAnimationFrame(draw); return; }
    }

    speedFactor = lerp(speedFactor, paused ? 0.12 : 1, 0.05);
    desat = lerp(desat, paused ? 1 : 0, 0.05);
    spin += 0.0032 * speedFactor;
    breath += 0.010 * speedFactor;

    ctx.clearRect(0, 0, width, height);

    const breathe = 1 + 0.05 * Math.sin(breath);
    const cosY = Math.cos(spin);
    const sinY = Math.sin(spin);

    const projected = particles.map((p) => {
      p.vx += (p.hx - p.x) * 0.0070;
      p.vy += (p.hy - p.y) * 0.0070;
      p.vz += (p.hz - p.z) * 0.0070;
      p.vx *= 0.955; p.vy *= 0.955; p.vz *= 0.955;
      p.x += p.vx; p.y += p.vy; p.z += p.vz;
      const rx = p.x * cosY + p.z * sinY;
      const rz = -p.x * sinY + p.z * cosY;
      const ry = p.y * breathe;
      const fov = 3.8;
      const scale = fov / (fov + rz * 0.35);
      const sx = cx + rx * scale * planetRadius;
      const sy = cy + ry * scale * planetRadius;
      const depth = (rz + 2.5) / 5.0;
      return { p, sx, sy, rz, depth, scale };
    });

    projected.sort((a, b) => a.rz - b.rz);

    const d = desat;
    for (const { p, sx, sy, depth, scale } of projected) {
      if (p.type === 'body') {
        // 钴蓝小粒子,在淡蓝/白卡上靠深浅表现立体(无发光,适配浅底)
        let red = 38 + p.colorT * 16 + depth * 46;
        let green = 64 + p.colorT * 18 + depth * 60;
        let blue = 178 + p.colorT * 22 + depth * 44;
        let alpha = 0.7 + depth * 0.3;
        if (d > 0.001) {
          red = lerp(red, GRAY[0], d * 0.62);
          green = lerp(green, GRAY[1], d * 0.62);
          blue = lerp(blue, GRAY[2], d * 0.62);
          alpha *= (1 - d * 0.12);
        }
        const size = Math.max(0.45, p.size * scale * (0.9 + depth * 0.6));
        ctx.beginPath();
        ctx.arc(sx, sy, size, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${Math.round(red)},${Math.round(Math.min(255, green))},${Math.round(Math.min(255, blue))},${alpha})`;
        ctx.fill();
        // 浅底不画发光(radial 加亮在白纸上无效),星球靠钴蓝粒子本身深浅表现立体
      } else {
        let alpha = (0.72 + depth * 0.28) * (0.78 + p.colorT * 0.22);
        let r = 42, g = 70, b = 195;
        if (d > 0.001) {
          r = lerp(r, GRAY[0], d * 0.62);
          g = lerp(g, GRAY[1], d * 0.62);
          b = lerp(b, GRAY[2], d * 0.62);
        }
        const size = Math.max(0.5, p.size * scale * 1.35);
        ctx.beginPath();
        ctx.arc(sx, sy, size, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${Math.round(r)},${Math.round(g)},${Math.round(b)},${Math.min(1.0, alpha)})`;
        ctx.fill();
        // 环粒子同样不画发光(浅底)
      }
    }

    raf = requestAnimationFrame(draw);
  };

  const start = () => {
    if (!built) build();
    resize();
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(draw);
  };

  // 点击星球 → 散开;并阻止冒泡,避免触发卡片的「切换监听」(切换监听点底部文字区)
  const onMouseDown = (e) => {
    const rect = canvas.getBoundingClientRect();
    scatter(e.clientX - rect.left, e.clientY - rect.top);
  };
  canvas.addEventListener('mousedown', onMouseDown);
  canvas.addEventListener('click', (e) => e.stopPropagation());

  if (banner) {
    const mo = new MutationObserver(() => {
      paused = banner.classList.contains('paused');
    });
    mo.observe(banner, { attributes: true, attributeFilter: ['class'] });
  }

  if (window.ResizeObserver) {
    new ResizeObserver(() => resize()).observe(canvas);
  } else {
    window.addEventListener('resize', resize);
  }

  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      cancelAnimationFrame(raf);
    } else {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(draw);
    }
  });

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true });
  } else {
    start();
  }
})();
