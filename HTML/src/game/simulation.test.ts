import test from 'node:test';
import assert from 'node:assert/strict';
import {
  MAX_PLOTS,
  SEED_PACK_COST,
  SEED_PACK_YIELD,
  advanceDay,
  brewPotion,
  buyDecor,
  buyOrSelectPotTheme,
  buySeedPack,
  createInitialGameState,
  expandGreenhousePlot,
  fulfillOrder,
  getNextPlotCost,
  harvestPlot,
  plantSeed,
  quickPlantToEmptyPlot,
  replayAlchemyStage,
  selectAlchemyStage,
  waterPlot,
} from './simulation';
import { ALCHEMY_STAGES, CHALLENGE_STAGE_IDS } from './alchemyStages';
import { composePinyin, getAvailableTones } from './pinyinEngine';

test('cultivation loop: plant, water, harvest increments pantry and deducts energy', () => {
  let state = createInitialGameState();
  const initialEnergy = state.energy;

  // Plot 5 starts empty -> plant 'f'
  state = plantSeed(state, 5, 'f', 'initial');
  assert.equal(state.plots[5].seedId, 'f');
  assert.equal(state.plots[5].growth, 0);

  // Water twice to reach maturity (growth 2)
  state = waterPlot(state, 5);
  assert.equal(state.plots[5].growth, 1);
  assert.equal(state.energy, initialEnergy - 1);

  state = waterPlot(state, 5);
  assert.equal(state.plots[5].growth, 2);
  assert.equal(state.energy, initialEnergy - 2);

  // Harvest adds +3 stock
  const beforeF = state.pantry.f ?? 0;
  state = harvestPlot(state, 5);
  assert.equal(state.pantry.f, beforeF + 3);
  assert.equal(state.plots[5].growth, 1);
});

test('alchemy loop: valid brew discovers syllable and earns discovery coins, invalid brew preserves pantry', () => {
  let state = createInitialGameState();
  const startCoins = state.coins;
  const startB = state.pantry.b;
  const startA = state.pantry.a;

  // Valid brew: b + a + 1声 -> bā
  const resValid = brewPotion(state, 'b', 'a', 1);
  state = resValid.state;
  assert.equal(resValid.outcome.valid, true);
  assert.equal(resValid.toneSupported, true);
  assert.equal(state.pantry.b, startB - 1);
  assert.equal(state.pantry.a, startA - 1);
  assert.ok(state.discoveredSyllables.includes('ba'));
  assert.equal(state.coins, startCoins + 12);
  assert.equal(state.potions.length, 1);
  assert.equal(state.potions[0].marked, 'bā');

  // Invalid empirical brew: b + ü -> fizzles and preserves pantry
  const beforeB = state.pantry.b;
  const beforeUmlaut = state.pantry['ü'];
  const resInvalid = brewPotion(state, 'b', 'ü', 1);
  assert.equal(resInvalid.outcome.valid, false);
  assert.equal(resInvalid.state.pantry.b, beforeB);
  assert.equal(resInvalid.state.pantry['ü'], beforeUmlaut);
});

test('villager order fulfillment awards gold, consumes potion, and progresses seasons', () => {
  let state = createInitialGameState();
  // First order is m + a + 3声 (mǎ 马)
  const firstOrder = state.orders[0];
  assert.equal(firstOrder.targetSyllable, 'ma');
  assert.equal(firstOrder.targetTone, 3);

  state = brewPotion(state, 'm', 'a', 3).state;
  const coinsBeforeDeliver = state.coins;

  state = fulfillOrder(state, firstOrder.id);
  assert.equal(state.ordersCompleted, 1);
  assert.equal(state.coins, coinsBeforeDeliver + firstOrder.rewardGold);
  assert.equal(state.potions.length, 0);
  assert.notEqual(state.orders[0].id, firstOrder.id);
});

test('decor upgrades and day transition apply auto-sprinkler and energy boost', () => {
  let state = createInitialGameState();
  // Default 40 coins is insufficient for 55-coin firefly lantern
  const blocked = buyDecor(state, 'firefly_lantern');
  assert.equal(blocked.maxEnergy, 6);

  state = { ...state, coins: 150 };

  state = buyDecor(state, 'firefly_lantern');
  assert.equal(state.maxEnergy, 10);
  assert.equal(state.energy, 10);

  state = buyDecor(state, 'auto_sprinkler');
  state = buyOrSelectPotTheme(state, 'celadon');
  assert.equal(state.potTheme, 'celadon');

  // Plot 3 starts at growth 0; advancing day with auto-sprinkler grows it to 1 and refills energy
  state = { ...state, energy: 2 };
  state = advanceDay(state);
  assert.equal(state.day, 2);
  assert.equal(state.energy, 10);
  assert.equal(state.plots[3].growth, 1);
});

test('greenhouse plot expansion adds empty pots up to MAX_PLOTS and enforces coin costs', () => {
  let state = createInitialGameState();
  assert.equal(state.plots.length, 6);
  assert.equal(getNextPlotCost(6), 25);

  // Starting 40 coins -> buy 7th pot (25 coins -> 15 coins left)
  state = expandGreenhousePlot(state);
  assert.equal(state.plots.length, 7);
  assert.equal(state.plots[6].id, 6);
  assert.equal(state.plots[6].seedId, null);
  assert.equal(state.coins, 15);

  // 8th pot costs 30 coins -> 15 coins should be blocked
  const blocked = expandGreenhousePlot(state);
  assert.equal(blocked.plots.length, 7);
  assert.equal(blocked.coins, 15);

  // Give plenty of coins and expand up to MAX_PLOTS (12)
  state = { ...state, coins: 500 };
  for (let i = 0; i < 10; i++) {
    state = expandGreenhousePlot(state);
  }
  assert.equal(state.plots.length, MAX_PLOTS);
  assert.equal(getNextPlotCost(state.plots.length), null);
});

test('shop seed pack replenishes pantry stock and quickPlantToEmptyPlot plants into first empty pot', () => {
  let state = createInitialGameState();
  assert.equal(state.pantry.p ?? 0, 0);

  // Buy seed pack for 'p' (costs 6 coins, +3 stock)
  const startCoins = state.coins;
  state = buySeedPack(state, 'p', 'initial');
  assert.equal(state.pantry.p, SEED_PACK_YIELD);
  assert.equal(state.coins, startCoins - SEED_PACK_COST);

  // Quick plant 'p' into first empty pot (plot id 5)
  state = quickPlantToEmptyPlot(state, 'p', 'initial');
  assert.equal(state.plots[5].seedId, 'p');
  assert.equal(state.plots[5].kind, 'initial');

  // Now all 6 pots are occupied -> quickPlantToEmptyPlot leaves plots unchanged
  const fullResult = quickPlantToEmptyPlot(state, 'o', 'final');
  assert.equal(fullResult.plots.filter((p) => p.seedId === 'o').length, 0);
});

test('three-part syllable alchemy brews j+i+a -> jiā (家) and fulfills villager order', () => {
  let state = createInitialGameState();
  const startJ = state.pantry.j;
  const startI = state.pantry.i;
  const startA = state.pantry.a;

  // Order 1 on Day 1 is j + i + a + 1声 -> jiā (家)
  const tripletOrder = state.orders[1];
  assert.equal(tripletOrder.targetSyllable, 'jia');
  assert.equal(tripletOrder.targetMedial, 'i');
  assert.equal(tripletOrder.targetTone, 1);

  const res = brewPotion(state, 'j', 'a', 1, 'i');
  assert.equal(res.outcome.valid, true);
  assert.equal(res.outcome.isTriplet, true);
  state = res.state;
  assert.equal(state.pantry.j, startJ - 1);
  assert.equal(state.pantry.i, startI - 1);
  assert.equal(state.pantry.a, startA - 1);
  assert.equal(state.potions[0].marked, 'jiā');
  assert.equal(state.potions[0].char, '家');

  state = fulfillOrder(state, tripletOrder.id);
  assert.equal(state.ordersCompleted, 1);
  assert.equal(state.potions.length, 0);
});

test('all 4 alchemy difficulty stages define valid Mandarin syllables and supported tones', () => {
  for (const stageId of CHALLENGE_STAGE_IDS) {
    const stage = ALCHEMY_STAGES.find((s) => s.id === stageId);
    assert.ok(stage, `Missing stage config for ${stageId}`);
    assert.equal(stage.tasks.length, 3);
    for (const task of stage.tasks) {
      const outcome = composePinyin(task.initial, task.final, task.medial);
      assert.equal(outcome.valid, true, `Task ${task.id} should compose validly`);
      assert.equal(outcome.syllable, task.syllable, `Task ${task.id} syllable mismatch`);
      assert.ok(
        getAvailableTones(outcome.syllable).includes(task.tone),
        `Task ${task.id} tone ${task.tone} should be supported for ${outcome.syllable}`
      );
      if (task.initial) {
        assert.ok(
          stage.featuredInitials.includes(task.initial),
          `Stage ${stageId} must feature initial ${task.initial}`
        );
      }
      assert.ok(
        stage.featuredFinals.includes(task.final),
        `Stage ${stageId} must feature final ${task.final}`
      );
    }
  }
});

test('alchemy difficulty stages progress tasks, preserve pantry via trial supply, and award first-clear gold once', () => {
  let state = createInitialGameState();
  const initialPantrySnapshot = { ...state.pantry };

  // Enter Stage 2 (三拼进阶: jia, duo, hua — note 'h' is not even unlocked on Day 1!)
  state = selectAlchemyStage(state, 'lvl2');
  assert.equal(state.activeStageId, 'lvl2');

  // Wrong potion in stage still brews cleanly without advancing step
  const wrong = brewPotion(state, 'j', 'a', 4, 'i');
  assert.equal(wrong.outcome.valid, true);
  assert.equal(wrong.stageMatched, false);
  state = wrong.state;
  assert.equal(state.stageProgress.lvl2, 0);

  // Solve all 3 tasks of Stage 2
  const lvl2 = ALCHEMY_STAGES.find((s) => s.id === 'lvl2')!;
  const coinsBeforeStageClear = state.coins;
  for (let idx = 0; idx < lvl2.tasks.length; idx++) {
    const t = lvl2.tasks[idx];
    const res = brewPotion(state, t.initial, t.final, t.tone, t.medial);
    assert.equal(res.stageMatched, true);
    state = res.state;
    assert.equal(state.stageProgress.lvl2, idx + 1);
  }

  assert.ok(state.clearedStages.includes('lvl2'));
  assert.ok(state.coins >= coinsBeforeStageClear + lvl2.rewardGold);
  // Pantry stock must remain untouched during trial stages
  assert.deepEqual(state.pantry, initialPantrySnapshot);

  // Replay Stage 2 -> resets step to 0, re-clearing does NOT duplicate clear gold
  state = replayAlchemyStage(state, 'lvl2');
  assert.equal(state.stageProgress.lvl2, 0);
  const coinsBeforeReplay = state.coins;
  for (const t of lvl2.tasks) {
    state = brewPotion(state, t.initial, t.final, t.tone, t.medial).state;
  }
  assert.equal(state.stageProgress.lvl2, 3);
  assert.equal(state.coins, coinsBeforeReplay);
});
