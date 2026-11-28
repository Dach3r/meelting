module Workspace
  ChecklistStep = Data.define(:key, :done, :available) do
    def initialize(available: true, **attributes)
      super(available: available, **attributes)
    end

    def pending?
      available && !done
    end
  end
end
