import { execSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';

const newVersion = process.argv[2];
const isForce = process.argv.includes('--force');

if (!newVersion) {
  console.error('❌ Usage: bun run release <version> [--force]');
  console.error('   Example: bun run release 0.1.4');
  process.exit(1);
}

// Validation
if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error('❌ Invalid version format. Use x.y.z (e.g., 1.0.0)');
  process.exit(1);
}

const root = process.cwd();
const tauriConfigPath = resolve(root, 'src-tauri/tauri.conf.json');
const packageJsonPath = resolve(root, 'package.json');
const tagName = `v${newVersion}`;

console.log(`🚀 Preparing release ${tagName}...`);

// Helper to run commands
const run = (cmd: string, ignoreError = false) => {
  try {
    execSync(cmd, { stdio: 'inherit' });
    return true;
  } catch (e) {
    if (!ignoreError) {
      console.error(`❌ Command failed: ${cmd}`);
      throw e;
    }
    return false;
  }
};

try {
  // 0. Check Git Status
  console.log('🔍 Checking git status...');
  try {
    const status = execSync('git status --porcelain').toString();
    if (status && !isForce) {
      // Allow if we are just version bumping files we are about to commit anyway?
      // But safer to ask for clean state or auto-stage.
      // For now, we assume we will stage tauri.conf.json and package.json.
      // Let's just warn.
      console.warn('⚠️  Warning: You have uncommitted changes.');
    }
  } catch (e) { /* ignore */ }

  // 1. Check for existing tags
  console.log('🔍 Checking for existing tags...');

  // Local tag
  const localTagExists = run(`git rev-parse ${tagName}`, true);
  if (localTagExists) {
    if (isForce) {
      console.log(`⚠️  Local tag ${tagName} exists. Deleting... (--force)`);
      run(`git tag -d ${tagName}`);
    } else {
      console.error(`❌ Error: Tag ${tagName} already exists locally.`);
      console.error(`   Use --force to overwrite: bun run release ${newVersion} --force`);
      process.exit(1);
    }
  }

  // Remote tag
  try {
    const remoteTags = execSync(`git ls-remote --tags origin ${tagName}`).toString();
    if (remoteTags.trim()) {
      if (isForce) {
        console.log(`⚠️  Remote tag ${tagName} exists. It will be overwritten on push. (--force)`);
      } else {
        console.error(`❌ Error: Tag ${tagName} already exists on remote.`);
        console.error(`   Use --force to overwrite: bun run release ${newVersion} --force`);
        process.exit(1);
      }
    }
  } catch (e) {
    console.warn("⚠️  Could not check remote tags (network issue?). Proceeding...");
  }

  // 2. Update tauri.conf.json
  const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf-8'));
  tauriConfig.version = newVersion;
  writeFileSync(tauriConfigPath, JSON.stringify(tauriConfig, null, 2));
  console.log('✅ Updated tauri.conf.json');

  // 3. Update package.json
  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  packageJson.version = newVersion;
  writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
  console.log('✅ Updated package.json');

  // 4. Git Commit and Tag
  console.log('📦 Committing and Tagging...');
  run('git add src-tauri/tauri.conf.json package.json');

  // Check if there is anything to commit
  try {
    execSync('git diff --cached --quiet');
    console.log('ℹ️  Nothing to commit (version might be same). proceeding to tag...');
  } catch {
    // Diff returns exit code 1 if there ARE changes (which is what we want)
    run(`git commit -m "chore: release ${tagName}"`, true); // Ignore if empty commit fails (unlikely due to add)
  }

  // Annotated tag: signed automatically when tag.gpgSign is enabled (scripts/setup-yubikey.ps1),
  // which the pre-push hook requires. Lightweight tags cannot carry a signature.
  run(`git tag -a ${tagName} -m "Release ${tagName}"`);

  console.log('⬆️  Pushing to GitHub...');
  run('git push'); // Push commits

  if (isForce) {
    run(`git push origin ${tagName} --force`);
  } else {
    run(`git push origin ${tagName}`);
  }

  console.log(`\n✨ Release ${tagName} triggered successfully! 🚀`);
  console.log('👉 Check progress at: https://github.com/ppundsh/WinNUT-Client-RS/actions');

} catch (error) {
  console.error('\n❌ Release failed.');
  process.exit(1);
}
