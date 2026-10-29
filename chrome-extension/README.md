# ViFastKey for Web

Xem `src/README.md`; build:

```bash
./chrome-extension/build.sh
# -> chrome-extension/dist
```

Test Playwright:

```bash
cd chrome-extension/test
npm install
xvfb-run -a npm test   # Linux; macOS có thể bỏ xvfb-run
```
