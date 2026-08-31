import {
  siC, siCplusplus, siCss, siDart, siDocker, siDotenv, siGit, siGnubash, siGo,
  siGraphql, siHelm, siHtml5, siJavascript, siJson, siKotlin, siLua, siMarkdown,
  siOpenjdk, siPhp, siPython, siReact, siRuby, siRust, siSass, siSvelte, siSwift,
  siTerraform, siToml, siTypescript, siVuedotjs, siYaml,
} from 'simple-icons';

import { iconKey } from '../shared/lib/icons';

export interface Brand { path: string; title: string }

/** Keyed by whole filename first, then by extension. */
const ICONS: Record<string, Brand> = {
  'chart.yaml': siHelm, 'values.yaml': siHelm, tpl: siHelm,

  ts: siTypescript, mts: siTypescript, cts: siTypescript,
  tsx: siReact, jsx: siReact,
  js: siJavascript, mjs: siJavascript, cjs: siJavascript,
  rs: siRust,
  py: siPython, pyi: siPython,
  go: siGo,
  rb: siRuby,
  java: siOpenjdk,
  kt: siKotlin, kts: siKotlin,
  swift: siSwift,
  php: siPhp,
  vue: siVuedotjs,
  svelte: siSvelte,
  dart: siDart,
  lua: siLua,
  c: siC, h: siC,
  cpp: siCplusplus, cc: siCplusplus, hpp: siCplusplus,
  html: siHtml5, htm: siHtml5,
  css: siCss,
  scss: siSass, sass: siSass,
  json: siJson, jsonc: siJson,
  md: siMarkdown, mdx: siMarkdown,
  yaml: siYaml, yml: siYaml,
  toml: siToml,
  env: siDotenv,
  sh: siGnubash, bash: siGnubash, zsh: siGnubash,
  graphql: siGraphql, gql: siGraphql,
  tf: siTerraform, tfvars: siTerraform,
  dockerfile: siDocker,
  gitignore: siGit, gitattributes: siGit, gitmodules: siGit,
};

export function brandFor(name: string): Brand | null {
  const base = (name.split('/').pop() ?? name).toLowerCase();
  return ICONS[base] ?? ICONS[iconKey(name)] ?? null;
}
