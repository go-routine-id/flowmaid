import { Config } from '@remotion/cli/config';

Config.setVideoImageFormat('jpeg');
Config.setOverwriteOutput(true);
Config.setChromiumOpenGlRenderer('angle');
Config.setConcurrency(4);
// Async loaders (fonts, SVG markup) can wait > 30 s for the static server while 4 tabs extract
// video frames; the default 30 s delayRender timeout failed one integration render.
Config.setDelayRenderTimeoutInMilliseconds(120000);
