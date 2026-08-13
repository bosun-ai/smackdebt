import core from '../core/main';
import local from './choice';
import library from 'external-library';

const late = require(moduleName);

export function app() {
  return core(local, library, late);
}
