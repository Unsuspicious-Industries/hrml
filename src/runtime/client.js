(function () {
  'use strict';

  function swapIntoTarget(target, swap, html) {
    if (!target) return;
    var el = document.querySelector(target);
    if (!el) return;

    if (swap === 'outerHTML') el.outerHTML = html;
    else if (swap === 'beforeend') el.insertAdjacentHTML('beforeend', html);
    else el.innerHTML = html;
  }

  function parseBody(response) {
    var type = response.headers.get('content-type') || '';
    if (type.indexOf('application/json') !== -1) return response.json();
    return response.text();
  }

  function sendRequest(url, method, data) {
    return fetch(url, {
      method: method,
      headers: {
        'Content-Type': 'application/json'
      },
      body: method === 'GET' ? undefined : JSON.stringify(data || {})
    }).then(parseBody);
  }

  document.addEventListener('click', function (e) {
    var btn = e.target.closest('[data-post]');
    if (!btn) return;
    e.preventDefault();

    var url = btn.getAttribute('data-post');
    var target = btn.getAttribute('data-target');
    var swap = btn.getAttribute('data-swap') || 'innerHTML';

    sendRequest(url, 'POST', {}).then(function (body) {
      swapIntoTarget(target, swap, typeof body === 'string' ? body : JSON.stringify(body));
    }).catch(function (err) {
      console.error('Request failed:', err);
    });
  });

  document.addEventListener('click', function (e) {
    var link = e.target.closest('[data-get]');
    if (!link) return;
    e.preventDefault();

    var url = link.getAttribute('data-get');
    var target = link.getAttribute('data-target');
    var swap = link.getAttribute('data-swap') || 'innerHTML';

    fetch(url).then(parseBody).then(function (body) {
      swapIntoTarget(target, swap, typeof body === 'string' ? body : JSON.stringify(body));
    }).catch(function (err) {
      console.error('Request failed:', err);
    });
  });

  document.addEventListener('submit', function (e) {
    var form = e.target.closest('[data-post]');
    if (!form) return;
    e.preventDefault();

    var url = form.getAttribute('data-post');
    var target = form.getAttribute('data-target');
    var swap = form.getAttribute('data-swap') || 'innerHTML';
    var data = {};
    new FormData(form).forEach(function (v, k) {
      data[k] = v;
    });

    sendRequest(url, 'POST', data).then(function (body) {
      swapIntoTarget(target, swap, typeof body === 'string' ? body : JSON.stringify(body));
      form.reset();
    }).catch(function (err) {
      console.error('Form failed:', err);
    });
  });

  document.addEventListener('change', function (e) {
    var cb = e.target;
    if (cb.type !== 'checkbox' || !cb.hasAttribute('data-post')) return;

    var url = cb.getAttribute('data-post');
    var target = cb.getAttribute('data-target');
    var swap = cb.getAttribute('data-swap') || 'outerHTML';

    sendRequest(url, 'POST', { checked: !!cb.checked }).then(function (body) {
      swapIntoTarget(target, swap, typeof body === 'string' ? body : JSON.stringify(body));
    }).catch(function (err) {
      console.error('Checkbox request failed:', err);
    });
  });

  function tryParseProps(input) {
    try {
      return JSON.parse(input || '{}');
    } catch (_e) {
      return {};
    }
  }

  function mountWasmWindows() {
    var windows = document.querySelectorAll('[data-wasm-module], [data-wasm-src]');

    windows.forEach(function (el) {
      if (el.dataset.wasmMounted === '1') return;
      el.dataset.wasmMounted = '1';

      var moduleName = el.getAttribute('data-wasm-module');
      var src = el.getAttribute('data-wasm-src');
      var exportName = el.getAttribute('data-wasm-export') || 'mount';
      var startName = el.getAttribute('data-wasm-start') || '';
      var props = tryParseProps(el.getAttribute('data-wasm-props'));

      (async function () {
        try {
          if (moduleName) {
            var mod = await import(moduleName);
            if (startName && typeof mod[startName] === 'function') {
              await mod[startName]();
            }
            if (typeof mod[exportName] === 'function') {
              await mod[exportName](el, props);
              return;
            }
            throw new Error('Missing export: ' + exportName);
          }

          if (src) {
            var bytes = await fetch(src).then(function (r) { return r.arrayBuffer(); });
            var instantiated = await WebAssembly.instantiate(bytes, {});
            var instance = instantiated.instance || instantiated;
            var fn = instance.exports[exportName] || instance.exports.default || instance.exports.main;
            if (typeof fn === 'function') {
              fn();
              if (!el.innerHTML.trim()) {
                el.innerHTML = '<div class="hrml-wasm-canvas">WASM mounted</div>';
              }
              return;
            }
            throw new Error('No callable wasm export found');
          }

          throw new Error('No data-wasm-module or data-wasm-src provided');
        } catch (err) {
          console.error('WASM mount failed:', err);
          el.innerHTML = '<pre class="hrml-wasm-error">WASM error: ' + String(err) + '</pre>';
        }
      })();
    });
  }

  document.addEventListener('DOMContentLoaded', mountWasmWindows);
})();
