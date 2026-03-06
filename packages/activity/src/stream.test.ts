import { describe, it, expect, vi } from "vitest";
import type { ActivityEvent } from "./stream.js";

async function loadStream() {
  vi.resetModules();
  return import("./stream.js");
}

function makeEvent(overrides?: Partial<ActivityEvent>): ActivityEvent {
  return {
    id: "evt-1",
    operation: "query",
    memory_id: null,
    details: {},
    created_at: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("subscribeToActivity", () => {
  it("returns an unsubscribe function", async () => {
    const { subscribeToActivity } = await loadStream();
    const unsub = subscribeToActivity(() => {});
    expect(typeof unsub).toBe("function");
  });
});

describe("emitActivity", () => {
  it("calls subscribed listeners with the event", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener = vi.fn();
    subscribeToActivity(listener);

    const event = makeEvent();
    emitActivity(event);

    expect(listener).toHaveBeenCalledOnce();
    expect(listener).toHaveBeenCalledWith(event);
  });

  it("calls multiple listeners", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener1 = vi.fn();
    const listener2 = vi.fn();
    subscribeToActivity(listener1);
    subscribeToActivity(listener2);

    emitActivity(makeEvent());

    expect(listener1).toHaveBeenCalledOnce();
    expect(listener2).toHaveBeenCalledOnce();
  });

  it("does not call unsubscribed listeners", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener = vi.fn();
    const unsub = subscribeToActivity(listener);

    unsub();
    emitActivity(makeEvent());

    expect(listener).not.toHaveBeenCalled();
  });

  it("continues calling other listeners when one throws", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const badListener = vi.fn(() => {
      throw new Error("listener failure");
    });
    const goodListener = vi.fn();

    subscribeToActivity(badListener);
    subscribeToActivity(goodListener);

    emitActivity(makeEvent());

    expect(badListener).toHaveBeenCalledOnce();
    expect(goodListener).toHaveBeenCalledOnce();
    expect(errorSpy).toHaveBeenCalled();

    errorSpy.mockRestore();
  });
});

describe("integration: multi-step lifecycle", () => {
  it("subscribe -> emit -> partial unsub -> emit -> verify", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listenerA = vi.fn();
    const listenerB = vi.fn();

    const unsubA = subscribeToActivity(listenerA);
    subscribeToActivity(listenerB);

    emitActivity(makeEvent({ id: "evt-1" }));
    expect(listenerA).toHaveBeenCalledTimes(1);
    expect(listenerB).toHaveBeenCalledTimes(1);

    unsubA();

    emitActivity(makeEvent({ id: "evt-2" }));
    expect(listenerA).toHaveBeenCalledTimes(1);
    expect(listenerB).toHaveBeenCalledTimes(2);
  });
});
