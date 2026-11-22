import { join } from "node:path"

export function configPath(home: string): string {
  return join(home, ".config", "lib.json")
}
