// The job a Character's entry names, by the class it belongs to: the game's five first jobs and
// every advancement after them, as the Status Bar's reader knows them (`JOBS` in
// crates/minerva-core/src/status_bar.rs). The Map Leaderboards' pages and a Character's page
// show each job beside its class's icon (site/art/jobs). Plain data, so a page's logic and its
// tests can take it.

/** A class, by the first job it starts from. */
export type JobClass = "beginner" | "warrior" | "magician" | "bowman" | "thief" | "pirate";

/** Each class's jobs, as the game names them. */
const CLASSES: Record<JobClass, string[]> = {
  beginner: ["Beginner"],
  warrior: ["Warrior", "Fighter", "Page", "Spearman", "Crusader", "White Knight", "Dragon Knight", "Hero", "Paladin", "Dark Knight"],
  magician: [
    "Magician",
    "Wizard (Fire, Poison)",
    "Wizard (Ice, Lightning)",
    "Wizard (F/P)",
    "Wizard (I/L)",
    "F/P Wizard",
    "I/L Wizard",
    "Cleric",
    "Mage (Fire, Poison)",
    "Mage (Ice, Lightning)",
    "Mage (F/P)",
    "Mage (I/L)",
    "F/P Mage",
    "I/L Mage",
    "Priest",
    "Arch Mage (Fire, Poison)",
    "Arch Mage (Ice, Lightning)",
    "F/P Arch Mage",
    "I/L Arch Mage",
    "Bishop",
  ],
  bowman: ["Bowman", "Archer", "Hunter", "Crossbowman", "Ranger", "Sniper", "Bowmaster", "Marksman"],
  thief: ["Thief", "Rogue", "Assassin", "Bandit", "Hermit", "Chief Bandit", "Night Lord", "Shadower"],
  pirate: ["Pirate", "Brawler", "Gunslinger", "Marauder", "Outlaw", "Buccaneer", "Corsair"],
};

const BY_JOB = new Map(Object.entries(CLASSES).flatMap(([cls, jobs]) => jobs.map((job) => [job, cls as JobClass] as const)));

/** The class `job` belongs to, or `null` for a job the game doesn't name (the Map Leaderboards
 *  take any job an upload names, up to 32 characters). */
export function classOf(job: string): JobClass | null {
  return BY_JOB.get(job) ?? null;
}
