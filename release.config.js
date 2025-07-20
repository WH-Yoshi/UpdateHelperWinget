// release.config.js
module.exports = {
  branches: ["main"],
  plugins: [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    [
      "@semantic-release/github",
      {
        assets: [
          // Ces assets sont pour la release GitHub, mais nous allons les uploader manuellement
          // via softprops/action-gh-release après le build Rust.
          // Nous laissons cette partie vide ou commentée car softprops gère l'upload.
        ],
      },
    ],
    [
      "@semantic-release/git",
      {
        assets: ["CHANGELOG.md", "Cargo.toml"],
        message:
          "chore(release): release <%= nextRelease.version %> [skip ci]\n\n<%= nextRelease.notes %>",
      },
    ],
  ],
};