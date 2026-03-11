import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ActivityEvent } from "./stream.js";

function makeEvent(overrides: Partial<ActivityEvent> = {}): ActivityEvent {
  return {
    id: "evt-1",
    operation: "create",
    memory_id: null,
    details: {},
    created_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("activity stream", () => {
  let subscribeToActivity: (typeof import("./stream.js"))["subscribeToActivity"];
  let emitActivity: (typeof import("./stream.js"))["emitActivity"];

  beforeEach(async () => {
    vi.resetModules();
    const mod = await import("./stream.js");
    subscribeToActivity = mod.subscribeToActivity;
    emitActivity = mod.emitActivity;
  });

  it("subscribeToActivity returns an unsubscribe function", () => {
    const listener = vi.fn();
    const unsub = subscribeToActivity(listener);
    expect(typeof unsub).toBe("function");
  });

  it("emitActivity calls subscribed listeners with the event", () => {
    const listener = vi.fn();
    subscribeToActivity(listener);

    const event = makeEvent();
    emitActivity(event);

    expect(listener).toHaveBeenCalledOnce();
    expect(listener).toHaveBeenCalledWith(event);
  });

  it("emitActivity calls multiple listeners", () => {
    const listener1 = vi.fn();
    const listener2 = vi.fn();
    subscribeToActivity(listener1);
    subscribeToActivity(listener2);

    const event = makeEvent();
    emitActivity(event);

    expect(listener1).toHaveBeenCalledWith(event);
    expect(listener2).toHaveBeenCalledWith(event);
  });

  it("does not call unsubscribed listeners", () => {
    const listener = vi.fn();
    const unsub = subscribeToActivity(listener);
    unsub();

    emitActivity(makeEvent());

    expect(listener).not.toHaveBeenCalled();
  });

  it("continues calling other listeners when one throws", () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const badListener = vi.fn(() => {
      throw new Error("boom");
    });
    const goodListener = vi.fn();

    subscribeToActivity(badListener);
    subscribeToActivity(goodListener);

    const event = makeEvent();
    emitActivity(event);

    expect(badListener).toHaveBeenCalledWith(event);
    expect(goodListener).toHaveBeenCalledWith(event);
    expect(errorSpy).toHaveBeenCalled();
    errorSpy.mockRestore();
  });

  it("integration: subscribe -> emit -> partial unsub -> emit -> verify", () => {
    const listener1 = vi.fn();
    const listener2 = vi.fn();

    const unsub1 = subscribeToActivity(listener1);
    subscribeToActivity(listener2);

    const event1 = makeEvent({ id: "evt-1" });
    emitActivity(event1);

    expect(listener1).toHaveBeenCalledWith(event1);
    expect(listener2).toHaveBeenCalledWith(event1);

    unsub1();

    const event2 = makeEvent({ id: "evt-2" });
    emitActivity(event2);

    expect(listener1).toHaveBeenCalledTimes(1);
    expect(listener2).toHaveBeenCalledTimes(2);
    expect(listener2).toHaveBeenCalledWith(event2);
  });
});
