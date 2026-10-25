import { getUsr } from './users.js'
export const greet = (id) => `oi, ${getUsr(id)}`
