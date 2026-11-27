module Workspace
  Problem = Data.define(:kind, :repository) do
    def initialize(kind:, repository: nil)
      super
    end
  end
end
