const formatReleaseLines = (releases) => {
  return releases
    .filter((release) => release.type !== "none")
    .map((release) => ` - ${release.name}@${release.newVersion}`)
    .join("\n");
};

module.exports = {
  /**
   * Conventional commit used when someone runs `changeset add`.
   */
  getAddMessage: async (changeset) => `chore(changeset): ${changeset.summary}`,
  /**
   * Conventional commit used by `changeset version`.
   */
  getVersionMessage: async (releasePlan) => {
    const releaseLines = formatReleaseLines(releasePlan.releases);
    return releaseLines.length
      ? `chore(release): publish\n\n${releaseLines}`
      : "chore(release): publish";
  },
};
