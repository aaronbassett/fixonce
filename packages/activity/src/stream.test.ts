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

// --- Supabase Realtime tests ---

const mockRemoveChannel = vi.fn();
const mockSubscribe = vi.fn().mockReturnThis();
const mockOn = vi.fn().mockReturnValue({ subscribe: mockSubscribe });
const mockChannel = vi.fn().mockReturnValue({ on: mockOn });

vi.mock("@fixonce/storage", () => ({
  createSupabaseClient: vi.fn(() => ({
    channel: mockChannel,
    removeChannel: mockRemoveChannel,
  })),
}));

describe("subscribeToActivityRealtime", () => {
  let subscribeToActivityRealtime: (typeof import("./stream.js"))["subscribeToActivityRealtime"];

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    // Re-setup mocks after resetModules since the mock factory is preserved
    mockRemoveChannel.mockClear();
    mockSubscribe.mockClear().mockReturnThis();
    mockOn.mockClear().mockReturnValue({ subscribe: mockSubscribe });
    mockChannel.mockClear().mockReturnValue({ on: mockOn });

    const mod = await import("./stream.js");
    subscribeToActivityRealtime = mod.subscribeToActivityRealtime;
  });

  it("creates a channel subscription on the activity_log table", () => {
    const listener = vi.fn();
    subscribeToActivityRealtime(listener);

    expect(mockChannel).toHaveBeenCalledWith(
      expect.stringMatching(/^activity-realtime-.+$/),
    );
    expect(mockOn).toHaveBeenCalledWith(
      "postgres_changes",
      { event: "INSERT", schema: "public", table: "activity_log" },
      expect.any(Function),
    );
    expect(mockSubscribe).toHaveBeenCalled();
  });

  it("uses unique channel names for each subscription", () => {
    const listener1 = vi.fn();
    const listener2 = vi.fn();

    subscribeToActivityRealtime(listener1);
    subscribeToActivityRealtime(listener2);

    expect(mockChannel).toHaveBeenCalledTimes(2);

    const firstName = mockChannel.mock.calls[0]?.[0] as string;
    const secondName = mockChannel.mock.calls[1]?.[0] as string;

    expect(firstName).toMatch(/^activity-realtime-.+$/);
    expect(secondName).toMatch(/^activity-realtime-.+$/);
    expect(firstName).not.toBe(secondName);
  });

  it("passes a status callback to subscribe for error handling", () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const listener = vi.fn();
    subscribeToActivityRealtime(listener);

    expect(mockSubscribe).toHaveBeenCalledWith(expect.any(Function));

    const statusCallback = mockSubscribe.mock.calls[0]?.[0] as (
      status: string,
      err?: Error,
    ) => void;

    const testError = new Error("connection failed");
    statusCallback("CHANNEL_ERROR", testError);
    expect(errorSpy).toHaveBeenCalledWith(
      "Realtime subscription error:",
      "CHANNEL_ERROR",
      testError,
    );

    errorSpy.mockClear();
    statusCallback("TIMED_OUT", undefined);
    expect(errorSpy).toHaveBeenCalledWith(
      "Realtime subscription error:",
      "TIMED_OUT",
      undefined,
    );

    errorSpy.mockClear();
    statusCallback("SUBSCRIBED");
    expect(errorSpy).not.toHaveBeenCalled();

    errorSpy.mockRestore();
  });

  it("returns an unsubscribe function that removes the channel", () => {
    const listener = vi.fn();
    const unsub = subscribeToActivityRealtime(listener);

    expect(typeof unsub).toBe("function");
    unsub();

    expect(mockRemoveChannel).toHaveBeenCalled();
  });

  it("calls the listener when a postgres_changes payload arrives", () => {
    const listener = vi.fn();
    subscribeToActivityRealtime(listener);

    // Extract the callback passed to .on()
    const onCallback = mockOn.mock.calls[0]?.[2] as (payload: {
      new: ActivityEvent;
    }) => void;
    expect(onCallback).toBeDefined();

    const event = makeEvent({ id: "realtime-evt" });
    onCallback({ new: event });

    expect(listener).toHaveBeenCalledOnce();
    expect(listener).toHaveBeenCalledWith(event);
  });

  it("catches errors thrown by the listener", () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const badListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    subscribeToActivityRealtime(badListener);

    const onCallback = mockOn.mock.calls[0]?.[2] as (payload: {
      new: ActivityEvent;
    }) => void;

    const event = makeEvent({ id: "err-evt" });
    onCallback({ new: event });

    expect(badListener).toHaveBeenCalledWith(event);
    expect(errorSpy).toHaveBeenCalled();
    errorSpy.mockRestore();
  });
});

describe("cross-source independence", () => {
  let subscribeToActivity: (typeof import("./stream.js"))["subscribeToActivity"];
  let emitActivity: (typeof import("./stream.js"))["emitActivity"];
  let subscribeToActivityRealtime: (typeof import("./stream.js"))["subscribeToActivityRealtime"];

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    mockRemoveChannel.mockClear();
    mockSubscribe.mockClear().mockReturnThis();
    mockOn.mockClear().mockReturnValue({ subscribe: mockSubscribe });
    mockChannel.mockClear().mockReturnValue({ on: mockOn });

    const mod = await import("./stream.js");
    subscribeToActivity = mod.subscribeToActivity;
    emitActivity = mod.emitActivity;
    subscribeToActivityRealtime = mod.subscribeToActivityRealtime;
  });

  it("in-process listeners do NOT receive Realtime events", () => {
    const inProcessListener = vi.fn();
    subscribeToActivity(inProcessListener);

    // Simulate a Realtime event arriving via the postgres_changes callback
    const realtimeListener = vi.fn();
    subscribeToActivityRealtime(realtimeListener);

    const onCallback = mockOn.mock.calls[0]?.[2] as (payload: {
      new: ActivityEvent;
    }) => void;

    const realtimeEvent = makeEvent({ id: "realtime-only" });
    onCallback({ new: realtimeEvent });

    // The Realtime listener receives it
    expect(realtimeListener).toHaveBeenCalledWith(realtimeEvent);

    // The in-process listener does NOT receive it
    expect(inProcessListener).not.toHaveBeenCalled();
  });

  it("Realtime listeners do NOT receive in-process emitActivity events", () => {
    const realtimeListener = vi.fn();
    subscribeToActivityRealtime(realtimeListener);

    const inProcessEvent = makeEvent({ id: "in-process-only" });
    emitActivity(inProcessEvent);

    // The Realtime listener is only called via the postgres_changes callback,
    // not via emitActivity, so it should not have been called
    expect(realtimeListener).not.toHaveBeenCalled();
  });

  it("both channels operate independently with their own listeners", () => {
    const inProcessListener = vi.fn();
    const realtimeListener = vi.fn();

    subscribeToActivity(inProcessListener);
    subscribeToActivityRealtime(realtimeListener);

    // Emit an in-process event
    const localEvent = makeEvent({ id: "local-evt" });
    emitActivity(localEvent);

    expect(inProcessListener).toHaveBeenCalledWith(localEvent);
    expect(realtimeListener).not.toHaveBeenCalled();

    // Simulate a Realtime event
    const onCallback = mockOn.mock.calls[0]?.[2] as (payload: {
      new: ActivityEvent;
    }) => void;
    const remoteEvent = makeEvent({ id: "remote-evt" });
    onCallback({ new: remoteEvent });

    expect(realtimeListener).toHaveBeenCalledWith(remoteEvent);
    expect(inProcessListener).toHaveBeenCalledTimes(1); // Still only called once (from the local event)
  });
});
