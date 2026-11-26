import { Composition } from 'remotion';
import { AIFL_TOTAL } from './aifl/Main';
import { ThemedFilm } from './themes/ThemedFilm';
import { FLOWMAID_TOTAL, FlowmaidPromo } from './flowmaid/Main';

export const Root: React.FC = () => {
  return (
    <>
      <Composition
        id="AiflPromo"
        component={ThemedFilm}
        defaultProps={{theme: 'ink-press'}}
        durationInFrames={AIFL_TOTAL}
        fps={30}
        width={1920}
        height={1080}
      />
      {/* flowmaid LinkedIn promo: 1080×1350 (4:5) @ 30 fps, silent master. Pass
          --props='{"guides":true}' to draw the safe-area / caption-band / LinkedIn-overlay guides;
          --props='{"cover":true}' --frame=1078 renders the thumbnail out/cover.png. */}
      <Composition
        id="FlowmaidPromo"
        component={FlowmaidPromo}
        defaultProps={{guides: false, cover: false}}
        durationInFrames={FLOWMAID_TOTAL}
        fps={30}
        width={1080}
        height={1350}
      />
    </>
  );
};
