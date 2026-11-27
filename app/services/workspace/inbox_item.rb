module Workspace
  InboxItem = Data.define(:kind, :title, :repository, :agent, :waiting_seconds, :flag, :target) do
    def initialize(flag: nil, **attributes)
      super(flag: flag, **attributes)
    end
  end
end
