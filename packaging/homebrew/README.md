# JumpChamp Homebrew Distribution Guide 🍺🍏

This directory contains the Homebrew packaging definitions for **JumpChamp**.

By publishing a custom Homebrew Tap, any macOS user can install the desktop application with a single command:

```bash
brew install --cask JunghunLeePhD/tap/jumpchamp
```

---

## 🛠️ Step 1: Create your Homebrew Tap on GitHub

Homebrew uses git repositories starting with `homebrew-` to manage third-party software packages (called "Taps").

1. Go to GitHub and create a new **public** repository named:
   ```text
   homebrew-tap
   ```
   *(Full repository URL will be `https://github.com/JunghunLeePhD/homebrew-tap`)*.

2. Clone your new `homebrew-tap` repository locally:
   ```bash
   git clone git@github.com:JunghunLeePhD/homebrew-tap.git
   cd homebrew-tap
   ```

3. Create the `Casks` directory:
   ```bash
   mkdir -p Casks
   ```

4. Copy `packaging/homebrew/Casks/jumpchamp.rb` from this repository into `homebrew-tap/Casks/`:
   ```bash
   cp /path/to/jumpchamp/packaging/homebrew/Casks/jumpchamp.rb Casks/
   ```

5. Commit and push:
   ```bash
   git add Casks/jumpchamp.rb
   git commit -m "feat: add jumpchamp cask v1.0.0"
   git push origin main
   ```

---

## 🚀 Step 2: How Friends Install JumpChamp on Mac

Once `homebrew-tap` is pushed to GitHub, share these commands with your friend:

### Direct Installation:
```bash
brew install --cask JunghunLeePhD/tap/jumpchamp
```

Homebrew will automatically:
1. Tap your repository `https://github.com/JunghunLeePhD/homebrew-tap`.
2. Detect if their Mac is Apple Silicon (M1/M2/M3/M4) or Intel.
3. Download the matching `.app.zip` from your GitHub Release.
4. Verify the SHA-256 checksum for security.
5. Extract and place `JumpChamp.app` into `/Applications`.

### ⚠️ First Launch Note (macOS Gatekeeper):
Because open-source builds without a paid Apple Developer certificate are flagged by macOS Gatekeeper on first launch, if macOS says *"JumpChamp is damaged and can't be opened"*, run:
```bash
xattr -cr /Applications/JumpChamp.app
```
*(Or Right-Click `JumpChamp.app` in `/Applications` -> click **Open** -> click **Open** on confirmation).*

---

## 🔄 Updating JumpChamp for Future Releases

When you publish a new version (e.g., `v1.1.0`):

1. Run the generator script in this repository:
   ```bash
   ./scripts/generate_brew_cask.sh v1.1.0
   ```
2. Copy the updated `packaging/homebrew/Casks/jumpchamp.rb` to your `homebrew-tap/Casks/` folder.
3. Commit and push the change to `homebrew-tap`.
4. Your friend can upgrade anytime with:
   ```bash
   brew upgrade --cask jumpchamp
   ```
