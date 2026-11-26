// Package A private helper: public/hero.mmd with the playground's syntax colours, typed line
// by line. The token array is copied from the approved effects preview ../fw/fx.html (CODE);
// joined, it is exactly the text of public/hero.mmd.
import React from 'react';
import { FONT, PAPER, SYNTAX } from '../../tokens';

type Tok = [keyof typeof SYNTAX | '', string];

export const CODE: Tok[][] = [
  [['header', 'flowchart'], ['', ' '], ['dir', 'TD']],
  [['', '  A'], ['pu', '(['], ['text', 'Request'], ['pu', '])'], ['', ' '], ['op', '-->'], ['', ' B'], ['pu', '{'], ['text', 'Auth?'], ['pu', '}']],
  [['', '  B '], ['op', '-->'], ['pu', '|'], ['str', 'yes'], ['pu', '|'], ['', ' C'], ['pu', '['], ['text', 'API Gateway'], ['pu', ']']],
  [['', '  B '], ['op', '-->'], ['pu', '|'], ['str', 'no'], ['pu', '|'], ['', ' D'], ['pu', '['], ['text', '401'], ['pu', ']']],
  [['', '  '], ['kw', 'subgraph'], ['', ' Services']],
  [['', '    C '], ['op', '-->'], ['', ' E'], ['pu', '[('], ['text', 'Cache'], ['pu', ')]']],
  [['', '    C '], ['op', '-->'], ['', ' F'], ['pu', '[('], ['text', 'Database'], ['pu', ')]']],
  [['', '  '], ['kw', 'end']],
  [['', '  E '], ['op', '-->'], ['', ' G'], ['pu', '(['], ['text', 'Response'], ['pu', '])']],
  [['', '  F '], ['op', '-->'], ['', ' G']],
];

export const lineLength = (i: number) => CODE[i].reduce((n, [, s]) => n + s.length, 0);

export const CODE_FONT_SIZE = 22;
export const CODE_LINE_H = 35;
export const CODE_PAD = 22;

/**
 * The code block. `typed[i]` = number of visible characters of line i (0..len). The caret
 * (accent, 3 px) sits after the last visible character of line `caretLine` when `caret` is on.
 */
export const HeroCode: React.FC<{ typed: number[]; caretLine: number; caret: boolean }> = ({ typed, caretLine, caret }) => (
  <div
    style={{
      position: 'absolute', left: 0, top: 0, padding: CODE_PAD, fontFamily: FONT.mono, fontWeight: 400, fontSize: CODE_FONT_SIZE,
      lineHeight: `${CODE_LINE_H}px`, whiteSpace: 'pre', color: PAPER.ink, fontVariantLigatures: 'none', fontFeatureSettings: '"calt" 0, "liga" 0',
    }}
  >
    {CODE.map((line, i) => {
      let left = typed[i] ?? 0;
      return (
        <div key={i} style={{ height: CODE_LINE_H }}>
          {line.map(([cls, s], k) => {
            if (left <= 0) return null;
            const part = s.slice(0, left);
            left -= part.length;
            return (
              <span key={k} style={cls ? { color: SYNTAX[cls] } : undefined}>
                {part}
              </span>
            );
          })}
          {caret && i === caretLine ? (
            <span
              style={{ display: 'inline-block', width: 3, height: '1.2em', marginLeft: 1, verticalAlign: '-0.25em', background: PAPER.accent }}
            />
          ) : null}
        </div>
      );
    })}
  </div>
);
