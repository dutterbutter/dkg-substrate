/*
 * Copyright 2022 Webb Technologies Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
// just a blueprint test using jest

import { BLOCK_TIME } from "../src/constants";
import { executeAfter, executeBefore } from "./utils/util";

function importTest(name: string, path: string) {
  describe(name, function () {
      require(path);
  });
}

describe('E2E Test Runner', function () {
  this.timeout(100 * BLOCK_TIME);
  before(async () => {
    await executeBefore({
      both: true,
    });
  })

  importTest('Proposer Set Update Proposal', './proposerSetUpdateProposal.test');
  importTest('Maximum Deposit Limit Update Proposal', './maxDepositLimitProposal.test');
  importTest('Minimum Withdrawal Limit Update Proposal', './minWithdrawalLimit.test');
  importTest('Rescue Tokens Proposal', './rescueTokensProposal.test');
  importTest('Wrapping Fee Update Proposal', './wrappingFeeUpdateProposal.test');
  importTest('Resource ID Update Proposal', './resourceIdUpdateProposal.test');
  importTest('Token Add & Remove Proposal', './tokenUpdateProposal.test');
  importTest('Anchor Update Proposal', './updateAnchorProposal.test');
  importTest('Governor Update / Refresh Proposal', './updateGovernor.test');
  // importTest('Threshold Update (Staking Chill)', './threshold.test');
  
  after(async () => {
    await executeAfter();
  });
});
  