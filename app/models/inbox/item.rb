module Inbox
  Item = Data.define(:id, :kind, :state, :title, :number, :cost_cents, :duration_seconds, :details) do
    def initialize(details: {}, cost_cents: nil, duration_seconds: nil, **attributes)
      super(details: details.freeze, cost_cents: cost_cents, duration_seconds: duration_seconds, **attributes)
    end

    def run?
      kind == :run
    end
    def pull_request?
      kind == :pull_request
    end
    def issue?
      kind == :issue
    end
    def awaiting_approval?
      state == :awaiting_approval
    end
    def running?
      state == :running
    end

    def detail(key)
      details.fetch(key)
    end
  end
end
