import { getUser } from './users.js'
export const greet = (id) => `oi, ${getUser(id)}`
