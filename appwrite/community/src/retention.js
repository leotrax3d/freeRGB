// D4 owner-review proposal. Only quota grace is consumed by cleanup today.
// Other limits require implementation, owner approval and a separate deployment.
export const retentionDays = Object.freeze({
  quotaGraceAfterExpiry: 1,
  ratingsAfterLastVote: 180,
  downloadReceiptsAfterCreation: 7,
  submissionsAndMessagesAfterSubmissionCreation: 30,
  auditAfterCreation: 30,
  blockedUsersAfterBlock: 30,
  anonymousUsersAfterLastActivity: 180,
  sessionMaximumLifetime: 30,
  functionExecutionLogs: 7,
  providerRequestLogs: 7,
  backupsMaximumLifetime: 30,
});

export const cleanupConfig = Object.freeze({
  intervalMinutes: 60,
  maxBatches: 20,
  batchSize: 100,
});
