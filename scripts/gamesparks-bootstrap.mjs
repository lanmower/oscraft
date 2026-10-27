#!/usr/bin/env node
// One-shot Gamesparks bootstrap: collection + manifest data entry.
// Call with no args; reads nothing from the user.
import { readFileSync, writeFileSync } from 'fs';

const manifestPath = 'scenarios/moba/manifest.json';
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

if (!manifest.gamesparks) {
  manifest.gamesparks = { collected: new Date().toISOString() };
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
  console.log('Manifest entry written to', manifestPath);
} else {
  console.log('Already bootstrapped; manifest.gamesparks present.');
}
