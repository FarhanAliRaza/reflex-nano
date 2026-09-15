// Register application components here, then npm run build:frontend and rebuild Rust.
import * as Themes from '@radix-ui/themes';
import { ClientCounter } from './custom.js';
export const registry = {'@radix-ui/themes': Themes, 'nano/demo': {ClientCounter}};
